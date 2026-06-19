use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::reminder::{CreateReminderDto, Reminder, ReminderWithContact, UpdateReminderDto},
};

pub async fn list_reminders(
    db: &PgPool,
    owner_id: Uuid,
    include_done: bool,
) -> Result<Vec<ReminderWithContact>> {
    sqlx::query_as::<_, ReminderWithContact>(
        "SELECT r.id, r.contact_id, r.kind, r.message, r.remind_at, r.recurrence, r.is_done,
                c.display_name AS contact_name, c.avatar_color AS contact_avatar_color
         FROM contacts.reminders r
         JOIN contacts.contacts c ON c.id = r.contact_id
         WHERE r.owner_id = $1 AND ($2 OR r.is_done = FALSE)
         ORDER BY r.remind_at ASC",
    )
    .bind(owner_id)
    .bind(include_done)
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)
}

pub async fn create_reminder(db: &PgPool, owner_id: Uuid, dto: &CreateReminderDto) -> Result<Reminder> {
    // Ensure the contact belongs to the owner before creating the reminder.
    let owns = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM contacts.contacts WHERE id = $1 AND owner_id = $2)",
    )
    .bind(dto.contact_id)
    .bind(owner_id)
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)?;
    if !owns {
        return Err(ContactsError::NotFound(format!("Contact {}", dto.contact_id)));
    }

    sqlx::query_as::<_, Reminder>(
        "INSERT INTO contacts.reminders (owner_id, contact_id, kind, message, remind_at, recurrence)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING *",
    )
    .bind(owner_id)
    .bind(dto.contact_id)
    .bind(dto.kind.as_deref().unwrap_or("follow_up"))
    .bind(&dto.message)
    .bind(dto.remind_at)
    .bind(dto.recurrence.as_deref().unwrap_or("none"))
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)
}

pub async fn update_reminder(
    db: &PgPool,
    owner_id: Uuid,
    id: Uuid,
    dto: &UpdateReminderDto,
) -> Result<Reminder> {
    let existing = sqlx::query_as::<_, Reminder>(
        "SELECT * FROM contacts.reminders WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Rappel {id}")))?;

    let message    = dto.message.as_ref().or(existing.message.as_ref());
    let remind_at  = dto.remind_at.unwrap_or(existing.remind_at);
    let recurrence = dto.recurrence.as_deref().unwrap_or(&existing.recurrence);
    let is_done    = dto.is_done.unwrap_or(existing.is_done);

    sqlx::query_as::<_, Reminder>(
        "UPDATE contacts.reminders SET message = $3, remind_at = $4, recurrence = $5, is_done = $6
         WHERE id = $1 AND owner_id = $2
         RETURNING *",
    )
    .bind(id).bind(owner_id).bind(message).bind(remind_at).bind(recurrence).bind(is_done)
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)
}

pub async fn delete_reminder(db: &PgPool, owner_id: Uuid, id: Uuid) -> Result<()> {
    let rows = sqlx::query("DELETE FROM contacts.reminders WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(owner_id)
        .execute(db)
        .await
        .map_err(ContactsError::Database)?
        .rows_affected();
    if rows == 0 {
        return Err(ContactsError::NotFound(format!("Rappel {id}")));
    }
    Ok(())
}

/// Count of reminders that are due now (for a sidebar badge).
pub async fn due_count(db: &PgPool, owner_id: Uuid) -> Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM contacts.reminders
         WHERE owner_id = $1 AND is_done = FALSE AND remind_at <= NOW()",
    )
    .bind(owner_id)
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)
}
