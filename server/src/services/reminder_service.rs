use kubuno_db::{new_id, params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::reminder::{CreateReminderDto, Reminder, ReminderWithContact, UpdateReminderDto},
    sync,
};

pub async fn list_reminders(
    db: &DbPool,
    owner_id: Uuid,
    include_done: bool,
) -> Result<Vec<ReminderWithContact>> {
    db.fetch_all_as::<ReminderWithContact>(
        "SELECT r.id, r.contact_id, r.kind, r.message, r.remind_at, r.recurrence, r.is_done, \
                c.display_name AS contact_name, c.avatar_color AS contact_avatar_color \
         FROM contacts.reminders r \
         JOIN contacts.contacts c ON c.id = r.contact_id \
         WHERE r.owner_id = $1 AND ($2 OR r.is_done = FALSE) \
         ORDER BY r.remind_at ASC",
        params![owner_id, include_done],
    )
    .await
    .map_err(ContactsError::Database)
}

async fn select_reminder(db: &DbPool, owner_id: Uuid, id: Uuid) -> Result<Reminder> {
    db.fetch_optional_as::<Reminder>(
        "SELECT * FROM contacts.reminders WHERE id = $1 AND owner_id = $2",
        params![id, owner_id],
    )
    .await
    .map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Rappel {id}")))
}

pub async fn create_reminder(db: &DbPool, owner_id: Uuid, dto: &CreateReminderDto) -> Result<Reminder> {
    let owns = db
        .fetch_scalar::<i64>(
            "SELECT COUNT(*) FROM contacts.contacts WHERE id = $1 AND owner_id = $2",
            params![dto.contact_id, owner_id],
        )
        .await
        .map_err(ContactsError::Database)?
        > 0;
    if !owns {
        return Err(ContactsError::NotFound(format!("Contact {}", dto.contact_id)));
    }

    let id = dto.id.unwrap_or_else(new_id);
    let now = chrono::Utc::now();
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_reminder_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "INSERT INTO contacts.reminders \
         (id, owner_id, contact_id, kind, message, remind_at, recurrence, change_seq, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        params![
            id, owner_id, dto.contact_id,
            dto.kind.as_deref().unwrap_or("follow_up"),
            dto.message.as_deref(),
            dto.remind_at,
            dto.recurrence.as_deref().unwrap_or("none"),
            seq, now,
        ],
    )
    .await
    .map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;

    select_reminder(db, owner_id, id).await
}

pub async fn update_reminder(
    db: &DbPool,
    owner_id: Uuid,
    id: Uuid,
    dto: &UpdateReminderDto,
) -> Result<Reminder> {
    let existing = select_reminder(db, owner_id, id).await?;

    let message    = dto.message.as_deref().or(existing.message.as_deref());
    let remind_at  = dto.remind_at.unwrap_or(existing.remind_at);
    let recurrence = dto.recurrence.as_deref().unwrap_or(&existing.recurrence);
    let is_done    = dto.is_done.unwrap_or(existing.is_done);

    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_reminder_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "UPDATE contacts.reminders SET message = $1, remind_at = $2, recurrence = $3, is_done = $4, change_seq = $5 \
         WHERE id = $6 AND owner_id = $7",
        params![message, remind_at, recurrence, is_done, seq, id, owner_id],
    )
    .await
    .map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;

    select_reminder(db, owner_id, id).await
}

pub async fn delete_reminder(db: &DbPool, owner_id: Uuid, id: Uuid) -> Result<()> {
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_reminder_seq(&mut tx).await.map_err(ContactsError::Database)?;
    let rows = tx
        .execute(
            "DELETE FROM contacts.reminders WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;
    if rows == 0 {
        tx.rollback().await.map_err(ContactsError::Database)?;
        return Err(ContactsError::NotFound(format!("Rappel {id}")));
    }
    sync::record_reminder_tombstone(&mut tx, id, owner_id, seq).await.map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(())
}

/// Count of reminders that are due now (for a sidebar badge).
pub async fn due_count(db: &DbPool, owner_id: Uuid) -> Result<i64> {
    db.fetch_scalar::<i64>(
        &format!(
            "SELECT {} FROM contacts.reminders WHERE owner_id = $1 AND is_done = FALSE AND remind_at <= $2",
            db.backend().count_bigint("*")
        ),
        params![owner_id, chrono::Utc::now()],
    )
    .await
    .map_err(ContactsError::Database)
}
