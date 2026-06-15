use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Group {
    pub id:         Uuid,
    pub owner_id:   Uuid,
    pub name:       String,
    pub color:      String,
    pub is_system:  bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateGroupDto {
    pub name:  String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateGroupDto {
    pub name:  Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupWithCount {
    pub id:            Uuid,
    pub owner_id:      Uuid,
    pub name:          String,
    pub color:         String,
    pub is_system:     bool,
    pub contact_count: i64,
    pub created_at:    DateTime<Utc>,
    pub updated_at:    DateTime<Utc>,
}
