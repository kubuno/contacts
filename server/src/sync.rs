//! Delta-sync plumbing shared by the services and the delta handler.
//!
//! The local-first pull (contacts / labels / groups / reminders) rests on a
//! monotonic `change_seq` per record and a tombstone per hard-deleted row. On
//! PostgreSQL that used to be a `SEQUENCE` plus `BEFORE UPDATE` / `AFTER DELETE`
//! triggers; here it is the portable [`kubuno_db::journal`] primitive, driven
//! from Rust at every write site. This module holds the literal table / domain
//! names those calls take — all `&'static str`, never request data — so the
//! write sites read uniformly and a rename happens in one place.
//!
//! Four entities are versioned: **contacts**, **labels**, **groups** and
//! **reminders**.
//!
//! * `contact_labels` changes bump their **contact** (label assignments ride
//!   inline in the contact delta).
//! * `group_members` changes bump their **group** (member ids ride inline in the
//!   group delta).
//! * Every entity is HARD-deleted (trash is a soft `is_trashed` flag on the
//!   contact); a permanent delete writes the matching tombstone.

use uuid::Uuid;

/// One shared counter table per schema; `next_seq` keys it by domain.
pub const CHANGE_COUNTER: &str = "contacts.change_counter";

pub const CONTACTS_TABLE: &str = "contacts.contacts";
pub const LABELS_TABLE: &str = "contacts.labels";
pub const GROUPS_TABLE: &str = "contacts.groups";
pub const REMINDERS_TABLE: &str = "contacts.reminders";

pub const CONTACT_TOMBSTONES: &str = "contacts.contact_tombstones";
pub const LABEL_TOMBSTONES: &str = "contacts.label_tombstones";
pub const GROUP_TOMBSTONES: &str = "contacts.group_tombstones";
pub const REMINDER_TOMBSTONES: &str = "contacts.reminder_tombstones";

/// Logical counter domains (the row keys in `change_counter`).
pub const CONTACT_DOMAIN: &str = "contacts";
pub const LABEL_DOMAIN: &str = "labels";
pub const GROUP_DOMAIN: &str = "groups";
pub const REMINDER_DOMAIN: &str = "reminders";

/// The next monotonic sequence for the **contacts** domain, taken inside `tx`.
pub async fn next_contact_seq(tx: &mut kubuno_db::DbTx) -> Result<i64, sqlx::Error> {
    kubuno_db::journal::next_seq(tx, CHANGE_COUNTER, CONTACT_DOMAIN).await
}

/// The next monotonic sequence for the **labels** domain, taken inside `tx`.
pub async fn next_label_seq(tx: &mut kubuno_db::DbTx) -> Result<i64, sqlx::Error> {
    kubuno_db::journal::next_seq(tx, CHANGE_COUNTER, LABEL_DOMAIN).await
}

/// The next monotonic sequence for the **groups** domain, taken inside `tx`.
pub async fn next_group_seq(tx: &mut kubuno_db::DbTx) -> Result<i64, sqlx::Error> {
    kubuno_db::journal::next_seq(tx, CHANGE_COUNTER, GROUP_DOMAIN).await
}

/// The next monotonic sequence for the **reminders** domain, taken inside `tx`.
pub async fn next_reminder_seq(tx: &mut kubuno_db::DbTx) -> Result<i64, sqlx::Error> {
    kubuno_db::journal::next_seq(tx, CHANGE_COUNTER, REMINDER_DOMAIN).await
}

/// Bumps a **contact** to a fresh sequence — the portable replacement for the
/// old child-triggered no-op `UPDATE`. Called after a label assignment changes.
/// Returns how many rows matched (`0` if the contact is gone).
pub async fn touch_contact(tx: &mut kubuno_db::DbTx, contact_id: Uuid) -> Result<u64, sqlx::Error> {
    kubuno_db::journal::touch(tx, CONTACTS_TABLE, CHANGE_COUNTER, CONTACT_DOMAIN, "id", contact_id)
        .await
        .map(|(_, affected)| affected)
}

/// Bumps a **group** to a fresh sequence. Called after a member changes.
pub async fn touch_group(tx: &mut kubuno_db::DbTx, group_id: Uuid) -> Result<u64, sqlx::Error> {
    kubuno_db::journal::touch(tx, GROUPS_TABLE, CHANGE_COUNTER, GROUP_DOMAIN, "id", group_id)
        .await
        .map(|(_, affected)| affected)
}

/// Writes a **contact** tombstone in the same transaction as its hard delete.
pub async fn record_contact_tombstone(
    tx: &mut kubuno_db::DbTx,
    id: Uuid,
    owner_id: Uuid,
    seq: i64,
) -> Result<(), sqlx::Error> {
    kubuno_db::journal::record_tombstone(tx, CONTACT_TOMBSTONES, id, owner_id, seq).await
}

/// Writes a **label** tombstone in the same transaction as its hard delete.
pub async fn record_label_tombstone(
    tx: &mut kubuno_db::DbTx,
    id: Uuid,
    owner_id: Uuid,
    seq: i64,
) -> Result<(), sqlx::Error> {
    kubuno_db::journal::record_tombstone(tx, LABEL_TOMBSTONES, id, owner_id, seq).await
}

/// Writes a **group** tombstone in the same transaction as its hard delete.
pub async fn record_group_tombstone(
    tx: &mut kubuno_db::DbTx,
    id: Uuid,
    owner_id: Uuid,
    seq: i64,
) -> Result<(), sqlx::Error> {
    kubuno_db::journal::record_tombstone(tx, GROUP_TOMBSTONES, id, owner_id, seq).await
}

/// Writes a **reminder** tombstone in the same transaction as its hard delete.
pub async fn record_reminder_tombstone(
    tx: &mut kubuno_db::DbTx,
    id: Uuid,
    owner_id: Uuid,
    seq: i64,
) -> Result<(), sqlx::Error> {
    kubuno_db::journal::record_tombstone(tx, REMINDER_TOMBSTONES, id, owner_id, seq).await
}
