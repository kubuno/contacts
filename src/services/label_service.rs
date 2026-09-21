use kubuno_db::{new_id, params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::label::{CreateLabelDto, Label, UpdateLabelDto},
    sync,
};

pub async fn list_labels(db: &DbPool, owner_id: Uuid) -> Result<Vec<Label>> {
    db.fetch_all_as::<Label>(
        "SELECT l.*, \
                (SELECT COUNT(*) FROM contacts.contact_labels cl \
                 JOIN contacts.contacts c ON c.id = cl.contact_id \
                 WHERE cl.label_id = l.id AND c.is_trashed = FALSE) AS contact_count \
         FROM contacts.labels l \
         WHERE l.owner_id = $1 \
         ORDER BY l.position ASC, l.name ASC",
        params![owner_id],
    )
    .await
    .map_err(ContactsError::Database)
}

/// Reselects one label with its (non-trashed) contact count — the portable
/// stand-in for `RETURNING`.
async fn select_label(db: &DbPool, owner_id: Uuid, label_id: Uuid) -> Result<Label> {
    db.fetch_optional_as::<Label>(
        "SELECT l.*, \
                (SELECT COUNT(*) FROM contacts.contact_labels cl \
                 JOIN contacts.contacts c ON c.id = cl.contact_id \
                 WHERE cl.label_id = l.id AND c.is_trashed = FALSE) AS contact_count \
         FROM contacts.labels l WHERE l.id = $1 AND l.owner_id = $2",
        params![label_id, owner_id],
    )
    .await
    .map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Étiquette {label_id}")))
}

pub async fn create_label(db: &DbPool, owner_id: Uuid, dto: &CreateLabelDto) -> Result<Label> {
    let name = dto.name.trim();
    if name.is_empty() {
        return Err(ContactsError::Validation("Le nom de l'étiquette est requis".into()));
    }
    let id = dto.id.unwrap_or_else(new_id);

    // Next position, computed in Rust (a `MAX(position)+1` subquery over the
    // same table in an INSERT is rejected by MySQL).
    let position: i32 = db
        .fetch_scalar::<i64>(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM contacts.labels WHERE owner_id = $1",
            params![owner_id],
        )
        .await
        .map_err(ContactsError::Database)? as i32;

    let now = chrono::Utc::now();
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_label_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "INSERT INTO contacts.labels (id, owner_id, name, color, icon, position, change_seq, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        params![
            id, owner_id, name,
            dto.color.as_deref().unwrap_or("#5f6368"),
            dto.icon.as_deref(), position, seq, now, now,
        ],
    )
    .await
    .map_err(map_unique)?;
    tx.commit().await.map_err(ContactsError::Database)?;

    select_label(db, owner_id, id).await
}

pub async fn update_label(
    db: &DbPool,
    owner_id: Uuid,
    label_id: Uuid,
    dto: &UpdateLabelDto,
) -> Result<Label> {
    let existing = select_label(db, owner_id, label_id).await?;

    let name  = dto.name.as_deref().unwrap_or(&existing.name);
    let color = dto.color.as_deref().unwrap_or(&existing.color);
    let icon  = dto.icon.as_deref().or(existing.icon.as_deref());
    let position = dto.position.unwrap_or(existing.position);
    let now = chrono::Utc::now();

    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_label_seq(&mut tx).await.map_err(ContactsError::Database)?;
    tx.execute(
        "UPDATE contacts.labels SET name = $1, color = $2, icon = $3, position = $4, \
         change_seq = $5, updated_at = $6 WHERE id = $7 AND owner_id = $8",
        params![name, color, icon, position, seq, now, label_id, owner_id],
    )
    .await
    .map_err(map_unique)?;
    tx.commit().await.map_err(ContactsError::Database)?;

    select_label(db, owner_id, label_id).await
}

pub async fn delete_label(db: &DbPool, owner_id: Uuid, label_id: Uuid) -> Result<()> {
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let seq = sync::next_label_seq(&mut tx).await.map_err(ContactsError::Database)?;
    let rows = tx
        .execute(
            "DELETE FROM contacts.labels WHERE id = $1 AND owner_id = $2",
            params![label_id, owner_id],
        )
        .await
        .map_err(ContactsError::Database)?;
    if rows == 0 {
        tx.rollback().await.map_err(ContactsError::Database)?;
        return Err(ContactsError::NotFound(format!("Étiquette {label_id}")));
    }
    sync::record_label_tombstone(&mut tx, label_id, owner_id, seq).await.map_err(ContactsError::Database)?;
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(())
}

/// Attaches a label to a set of contacts (idempotent), scoped to the owner. Each
/// touched contact is bumped so its inline `label_ids` re-sync.
pub async fn add_label_to_contacts(
    db: &DbPool,
    owner_id: Uuid,
    label_id: Uuid,
    contact_ids: &[Uuid],
) -> Result<u64> {
    if contact_ids.is_empty() {
        return Ok(0);
    }
    let owns = db
        .fetch_scalar::<i64>(
            "SELECT COUNT(*) FROM contacts.labels WHERE id = $1 AND owner_id = $2",
            params![label_id, owner_id],
        )
        .await
        .map_err(ContactsError::Database)?
        > 0;
    if !owns {
        return Err(ContactsError::NotFound(format!("Étiquette {label_id}")));
    }

    let backend = db.backend();
    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let mut affected = 0u64;
    let now = chrono::Utc::now();
    for &cid in contact_ids {
        // Only assign the label to a contact the owner really holds.
        let owns_contact = tx
            .fetch_optional_scalar::<i64>(
                "SELECT 1 FROM contacts.contacts WHERE id = $1 AND owner_id = $2",
                params![cid, owner_id],
            )
            .await
            .map_err(ContactsError::Database)?
            .is_some();
        if !owns_contact {
            continue;
        }
        let sql = format!(
            "INSERT {}INTO contacts.contact_labels (label_id, contact_id, added_at) VALUES ($1, $2, $3){}",
            backend.insert_ignore_prefix(),
            backend.on_conflict_do_nothing(&["label_id", "contact_id"]),
        );
        let rows = tx
            .execute(&sql, params![label_id, cid, now])
            .await
            .map_err(ContactsError::Database)?;
        if rows > 0 {
            sync::touch_contact(&mut tx, cid).await.map_err(ContactsError::Database)?;
            affected += rows;
        }
    }
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(affected)
}

pub async fn remove_label_from_contacts(
    db: &DbPool,
    owner_id: Uuid,
    label_id: Uuid,
    contact_ids: &[Uuid],
) -> Result<u64> {
    if contact_ids.is_empty() {
        return Ok(0);
    }
    // Confirm the label is the owner's before touching the join table.
    let owns = db
        .fetch_scalar::<i64>(
            "SELECT COUNT(*) FROM contacts.labels WHERE id = $1 AND owner_id = $2",
            params![label_id, owner_id],
        )
        .await
        .map_err(ContactsError::Database)?
        > 0;
    if !owns {
        return Ok(0);
    }

    let mut tx = db.begin().await.map_err(ContactsError::Database)?;
    let mut affected = 0u64;
    for &cid in contact_ids {
        let rows = tx
            .execute(
                "DELETE FROM contacts.contact_labels WHERE label_id = $1 AND contact_id = $2",
                params![label_id, cid],
            )
            .await
            .map_err(ContactsError::Database)?;
        if rows > 0 {
            sync::touch_contact(&mut tx, cid).await.map_err(ContactsError::Database)?;
            affected += rows;
        }
    }
    tx.commit().await.map_err(ContactsError::Database)?;
    Ok(affected)
}

/// Returns the label ids attached to a single contact.
pub async fn labels_for_contact(db: &DbPool, contact_id: Uuid) -> Result<Vec<Uuid>> {
    let rows = db
        .fetch_all_as::<LabelIdRow>(
            "SELECT label_id FROM contacts.contact_labels WHERE contact_id = $1",
            params![contact_id],
        )
        .await
        .map_err(ContactsError::Database)?;
    Ok(rows.into_iter().map(|r| r.label_id).collect())
}

#[derive(sqlx::FromRow)]
struct LabelIdRow {
    label_id: Uuid,
}

fn map_unique(e: sqlx::Error) -> ContactsError {
    match e {
        sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
            ContactsError::Conflict("Une étiquette de ce nom existe déjà".into())
        }
        other => ContactsError::Database(other),
    }
}
