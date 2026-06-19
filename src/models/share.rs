use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Share {
    pub id:           Uuid,
    pub owner_id:     Uuid,
    pub contact_id:   Option<Uuid>,
    pub group_id:     Option<Uuid>,
    pub token:        String,
    pub permission:   String,
    pub expires_at:   Option<DateTime<Utc>>,
    pub max_accesses: Option<i32>,
    pub access_count: i32,
    pub created_at:   DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateShareDto {
    pub contact_id:  Option<Uuid>,
    pub group_id:    Option<Uuid>,
    pub expires_in_days: Option<i64>,
    pub max_accesses:    Option<i32>,
    pub password:        Option<String>,
}
