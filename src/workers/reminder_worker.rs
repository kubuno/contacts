use std::time::Duration;

use chrono::{DateTime, Datelike, Utc};
use kubuno_db::{params, DbPool};
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
pub async fn run(db: DbPool, settings: Settings) {
    // Small initial delay so the module finishes registering with the core.
    tokio::time::sleep(Duration::from_secs(15)).await;
    loop {
        if let Err(e) = tick(&db, &settings).await {
            tracing::warn!(error = %e, "Cycle du worker de rappels en erreur");
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

/// The same instant one year later. February 29th has no counterpart in a
/// common year, so it falls back to the same number of days ahead.
fn next_year(at: DateTime<Utc>) -> DateTime<Utc> {
    at.with_year(at.year() + 1)
        .unwrap_or_else(|| at + chrono::Duration::days(365))
}

async fn tick(db: &DbPool, settings: &Settings) -> Result<(), sqlx::Error> {
    let now = Utc::now();

    // List the candidates, then claim each one on its own. The claim is the
    // UPDATE itself: it only touches a row that is still unnotified, so the
    // number of rows it changed IS the proof of ownership — exactly one worker
    // can see 1. This guard-column form is also the only one every engine can
    // express: neither SQLite nor MariaDB offers `SKIP LOCKED`.
    let due: Vec<DueReminder> = db
        .fetch_all_as::<DueReminder>(
            "SELECT r.id, r.owner_id, r.contact_id, r.kind, r.message, r.recurrence, r.remind_at, \
                    c.display_name AS contact_name \
             FROM contacts.reminders r \
             JOIN contacts.contacts c ON c.id = r.contact_id \
             WHERE r.is_done = FALSE AND r.notified_at IS NULL AND r.remind_at <= $1 \
             ORDER BY r.remind_at ASC \
             LIMIT 50",
            params![now],
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Rappels : lecture des échéances impossible");
            e
        })?;

    for r in due {
        // A fresh seq so the fired reminder re-syncs to local-first clients (the
        // old BEFORE UPDATE trigger did this). Taken on the pool; a gap left when
        // another worker wins the claim is harmless.
        let seq = kubuno_db::journal::next_seq_on_pool(
            db, crate::sync::CHANGE_COUNTER, crate::sync::REMINDER_DOMAIN,
        )
        .await?;

        // One statement both claims the reminder and leaves it in its final
        // state. A yearly reminder is moved to next year (its old date is part
        // of the guard), a one-shot one is stamped notified.
        let claimed = if r.recurrence == "yearly" {
            db.execute(
                "UPDATE contacts.reminders SET remind_at = $1, change_seq = $2 \
                 WHERE id = $3 AND is_done = FALSE AND notified_at IS NULL AND remind_at = $4",
                params![next_year(r.remind_at), seq, r.id, r.remind_at],
            )
            .await
        } else {
            db.execute(
                "UPDATE contacts.reminders SET notified_at = $1, change_seq = $2 \
                 WHERE id = $3 AND is_done = FALSE AND notified_at IS NULL",
                params![now, seq, r.id],
            )
            .await
        }
        .map_err(|e| {
            tracing::error!(error = %e, reminder_id = %r.id, "Rappels : réservation impossible");
            e
        })?;
        if claimed != 1 {
            // Another worker got there first, or the reminder was closed
            // between the two statements. Not ours to fire.
            continue;
        }

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
    }
    Ok(())
}
