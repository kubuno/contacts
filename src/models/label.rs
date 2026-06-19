use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Label {
    pub id:            Uuid,
    pub owner_id:      Uuid,
    pub name:          String,
    pub color:         String,
    pub icon:          Option<String>,
    pub is_system:     bool,
    pub position:      i32,
    #[sqlx(default)]
    pub contact_count: i64,
    pub created_at:    DateTime<Utc>,
    pub updated_at:    DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateLabelDto {
    pub name:  String,
    pub color: Option<String>,
    pub icon:  Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateLabelDto {
    pub name:     Option<String>,
    pub color:    Option<String>,
    pub icon:     Option<String>,
    pub position: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LabelMembersDto {
    pub contact_ids: Vec<Uuid>,
}
