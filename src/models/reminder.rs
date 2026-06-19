use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Reminder {
    pub id:          Uuid,
    pub owner_id:    Uuid,
    pub contact_id:  Uuid,
    pub kind:        String,
    pub message:     Option<String>,
    pub remind_at:   DateTime<Utc>,
    pub recurrence:  String,
    pub is_done:     bool,
    pub notified_at: Option<DateTime<Utc>>,
    pub created_at:  DateTime<Utc>,
}

/// A reminder joined with the contact's display name and avatar color, for the
/// reminders list UI.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ReminderWithContact {
    pub id:                  Uuid,
    pub contact_id:          Uuid,
    pub kind:                String,
    pub message:             Option<String>,
    pub remind_at:           DateTime<Utc>,
    pub recurrence:          String,
    pub is_done:             bool,
    pub contact_name:        String,
    pub contact_avatar_color: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateReminderDto {
    pub contact_id: Uuid,
    pub kind:       Option<String>,
    pub message:    Option<String>,
    pub remind_at:  DateTime<Utc>,
    pub recurrence: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateReminderDto {
    pub message:    Option<String>,
    pub remind_at:  Option<DateTime<Utc>>,
    pub recurrence: Option<String>,
    pub is_done:    Option<bool>,
}
