use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ContactsError, Result},
    models::label::{CreateLabelDto, Label, UpdateLabelDto},
};

pub async fn list_labels(db: &PgPool, owner_id: Uuid) -> Result<Vec<Label>> {
    sqlx::query_as::<_, Label>(
        "SELECT l.*,
                (SELECT COUNT(*) FROM contacts.contact_labels cl
                 JOIN contacts.contacts c ON c.id = cl.contact_id
                 WHERE cl.label_id = l.id AND c.is_trashed = FALSE) AS contact_count
         FROM contacts.labels l
         WHERE l.owner_id = $1
         ORDER BY l.position ASC, l.name ASC",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)
}

pub async fn create_label(db: &PgPool, owner_id: Uuid, dto: &CreateLabelDto) -> Result<Label> {
    let name = dto.name.trim();
    if name.is_empty() {
        return Err(ContactsError::Validation("Le nom de l'étiquette est requis".into()));
    }
    sqlx::query_as::<_, Label>(
        "INSERT INTO contacts.labels (owner_id, name, color, icon, position)
         VALUES ($1, $2, $3, $4,
                 COALESCE((SELECT MAX(position) + 1 FROM contacts.labels WHERE owner_id = $1), 0))
         RETURNING *, 0::bigint AS contact_count",
    )
    .bind(owner_id)
    .bind(name)
    .bind(dto.color.as_deref().unwrap_or("#5f6368"))
    .bind(&dto.icon)
    .fetch_one(db)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
            ContactsError::Conflict("Une étiquette de ce nom existe déjà".into())
        }
        other => ContactsError::Database(other),
    })
}

pub async fn update_label(
    db: &PgPool,
    owner_id: Uuid,
    label_id: Uuid,
    dto: &UpdateLabelDto,
) -> Result<Label> {
    let existing = sqlx::query_as::<_, Label>(
        "SELECT *, 0::bigint AS contact_count FROM contacts.labels WHERE id = $1 AND owner_id = $2",
    )
    .bind(label_id)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .map_err(ContactsError::Database)?
    .ok_or_else(|| ContactsError::NotFound(format!("Étiquette {label_id}")))?;

    let name  = dto.name.as_deref().unwrap_or(&existing.name);
    let color = dto.color.as_deref().unwrap_or(&existing.color);
    let icon  = dto.icon.as_ref().or(existing.icon.as_ref());
    let position = dto.position.unwrap_or(existing.position);

    sqlx::query_as::<_, Label>(
        "UPDATE contacts.labels SET name = $3, color = $4, icon = $5, position = $6
         WHERE id = $1 AND owner_id = $2
         RETURNING *,
                   (SELECT COUNT(*) FROM contacts.contact_labels cl WHERE cl.label_id = $1) AS contact_count",
    )
    .bind(label_id).bind(owner_id).bind(name).bind(color).bind(icon).bind(position)
    .fetch_one(db)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
            ContactsError::Conflict("Une étiquette de ce nom existe déjà".into())
        }
        other => ContactsError::Database(other),
    })
}

pub async fn delete_label(db: &PgPool, owner_id: Uuid, label_id: Uuid) -> Result<()> {
    let rows = sqlx::query("DELETE FROM contacts.labels WHERE id = $1 AND owner_id = $2")
        .bind(label_id)
        .bind(owner_id)
        .execute(db)
        .await
        .map_err(ContactsError::Database)?
        .rows_affected();
    if rows == 0 {
        return Err(ContactsError::NotFound(format!("Étiquette {label_id}")));
    }
    Ok(())
}

/// Attaches a label to a set of contacts (idempotent), scoped to the owner.
pub async fn add_label_to_contacts(
    db: &PgPool,
    owner_id: Uuid,
    label_id: Uuid,
    contact_ids: &[Uuid],
) -> Result<u64> {
    if contact_ids.is_empty() {
        return Ok(0);
    }
    // Verify the label belongs to the owner before touching the join table.
    let owns = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM contacts.labels WHERE id = $1 AND owner_id = $2)",
    )
    .bind(label_id)
    .bind(owner_id)
    .fetch_one(db)
    .await
    .map_err(ContactsError::Database)?;
    if !owns {
        return Err(ContactsError::NotFound(format!("Étiquette {label_id}")));
    }

    let rows = sqlx::query(
        "INSERT INTO contacts.contact_labels (label_id, contact_id)
         SELECT $1, c.id FROM contacts.contacts c
         WHERE c.id = ANY($2) AND c.owner_id = $3
         ON CONFLICT DO NOTHING",
    )
    .bind(label_id)
    .bind(contact_ids)
    .bind(owner_id)
    .execute(db)
    .await
    .map_err(ContactsError::Database)?
    .rows_affected();
    Ok(rows)
}

pub async fn remove_label_from_contacts(
    db: &PgPool,
    owner_id: Uuid,
    label_id: Uuid,
    contact_ids: &[Uuid],
) -> Result<u64> {
    if contact_ids.is_empty() {
        return Ok(0);
    }
    let rows = sqlx::query(
        "DELETE FROM contacts.contact_labels cl
         USING contacts.labels l
         WHERE cl.label_id = l.id AND l.owner_id = $3
           AND cl.label_id = $1 AND cl.contact_id = ANY($2)",
    )
    .bind(label_id)
    .bind(contact_ids)
    .bind(owner_id)
    .execute(db)
    .await
    .map_err(ContactsError::Database)?
    .rows_affected();
    Ok(rows)
}

/// Returns the label ids attached to a single contact.
pub async fn labels_for_contact(db: &PgPool, contact_id: Uuid) -> Result<Vec<Uuid>> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT label_id FROM contacts.contact_labels WHERE contact_id = $1",
    )
    .bind(contact_id)
    .fetch_all(db)
    .await
    .map_err(ContactsError::Database)
}
