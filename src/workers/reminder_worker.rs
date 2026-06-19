use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::Settings, services::core_client};

/// One due reminder picked up by the worker.
#[derive(sqlx::FromRow)]
struct DueReminder {
    id:           Uuid,
    owner_id:     Uuid,
    contact_id:   Uuid,
    kind:         String,
    message:      Option<String>,
    recurrence:   String,
    remind_at:    DateTime<Utc>,
    contact_name: String,
}

/// Periodically fires due reminders by delivering a targeted WebSocket
/// notification to the owner via the core, then either marks the reminder as
/// notified or rolls a yearly reminder forward to next year.
pub async fn run(db: PgPool, settings: Settings) {
    // Small initial delay so the module finishes registering with the core.
    tokio::time::sleep(Duration::from_secs(15)).await;
    loop {
        if let Err(e) = tick(&db, &settings).await {
            tracing::warn!(error = %e, "Cycle du worker de rappels en erreur");
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

async fn tick(db: &PgPool, settings: &Settings) -> Result<(), sqlx::Error> {
    // Claim due reminders atomically with SKIP LOCKED so concurrent instances
    // never fire the same reminder twice.
    let due: Vec<DueReminder> = sqlx::query_as::<_, DueReminder>(
        "SELECT r.id, r.owner_id, r.contact_id, r.kind, r.message, r.recurrence, r.remind_at,
                c.display_name AS contact_name
         FROM contacts.reminders r
         JOIN contacts.contacts c ON c.id = r.contact_id
         WHERE r.is_done = FALSE AND r.notified_at IS NULL AND r.remind_at <= NOW()
         ORDER BY r.remind_at ASC
         LIMIT 50
         FOR UPDATE OF r SKIP LOCKED",
    )
    .fetch_all(db)
    .await?;

    for r in due {
        let title = match r.kind.as_str() {
            "birthday" => format!("🎂 Anniversaire de {}", r.contact_name),
            _ => format!("Rappel : {}", r.contact_name),
        };
        let body = r.message.clone().unwrap_or_default();
        let event = core_client::user_notification(
            &[r.owner_id],
            "contacts.reminder",
            serde_json::json!({
                "reminder_id": r.id.to_string(),
                "contact_id":  r.contact_id.to_string(),
                "kind":        r.kind,
                "title":       title,
                "body":        body,
            }),
        );
        core_client::publish(settings, event).await;

        if r.recurrence == "yearly" {
            // Roll forward to next year and re-arm.
            sqlx::query(
                "UPDATE contacts.reminders
                 SET remind_at = $2 + INTERVAL '1 year', notified_at = NULL
                 WHERE id = $1",
            )
            .bind(r.id)
            .bind(r.remind_at)
            .execute(db)
            .await?;
        } else {
            sqlx::query("UPDATE contacts.reminders SET notified_at = NOW() WHERE id = $1")
                .bind(r.id)
                .execute(db)
                .await?;
        }
    }
    Ok(())
}
