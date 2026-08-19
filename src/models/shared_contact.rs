//! The instance's shared address book: outside contacts kept by the
//! administration and readable by everyone.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SharedContact {
    pub id:           Uuid,
    pub display_name: String,
    pub organization: Option<String>,
    pub job_title:    Option<String>,
    pub email:        Option<String>,
    pub phone:        Option<String>,
    pub notes:        Option<String>,
    pub updated_by:   Option<Uuid>,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSharedContactDto {
    pub display_name: String,
    pub organization: Option<String>,
    pub job_title:    Option<String>,
    pub email:        Option<String>,
    pub phone:        Option<String>,
    pub notes:        Option<String>,
}

/// Every field optional: an absent field is left alone, an explicit empty string
/// clears it (the handler normalises "" to NULL).
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSharedContactDto {
    pub display_name: Option<String>,
    pub organization: Option<String>,
    pub job_title:    Option<String>,
    pub email:        Option<String>,
    pub phone:        Option<String>,
    pub notes:        Option<String>,
}
