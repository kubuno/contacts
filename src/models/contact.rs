use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Contact {
    pub id:              Uuid,
    pub owner_id:        Uuid,

    pub given_name:      Option<String>,
    pub middle_name:     Option<String>,
    pub family_name:     Option<String>,
    pub name_prefix:     Option<String>,
    pub name_suffix:     Option<String>,
    pub nickname:        Option<String>,
    pub display_name:    String,

    pub organization:    Option<String>,
    pub department:      Option<String>,
    pub job_title:       Option<String>,

    pub avatar_path:     Option<String>,
    pub avatar_color:    String,

    pub emails:          sqlx::types::Json<Vec<ContactField>>,
    pub phones:          sqlx::types::Json<Vec<ContactField>>,
    pub addresses:       sqlx::types::Json<Vec<AddressField>>,
    pub urls:            sqlx::types::Json<Vec<ContactField>>,
    pub dates:           sqlx::types::Json<Vec<DateField>>,
    pub relations:       sqlx::types::Json<Vec<ContactField>>,
    pub instant_messages: sqlx::types::Json<Vec<ContactField>>,
    pub custom_fields:   sqlx::types::Json<Vec<CustomField>>,

    pub notes:           Option<String>,

    pub is_starred:      bool,
    pub is_trashed:      bool,
    pub trashed_at:      Option<DateTime<Utc>>,
    pub kubuno_user_id:  Option<Uuid>,

    pub vcard_uid:       String,
    pub etag:            String,
    pub import_source:   String,

    pub created_at:      DateTime<Utc>,
    pub updated_at:      DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactField {
    pub label: Option<String>,
    pub value: String,
    #[serde(rename = "type", default)]
    pub field_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressField {
    pub label:    Option<String>,
    #[serde(rename = "type", default)]
    pub field_type: String,
    pub street:   Option<String>,
    pub city:     Option<String>,
    pub region:   Option<String>,
    pub postcode: Option<String>,
    pub country:  Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateField {
    pub label: Option<String>,
    #[serde(rename = "type", default)]
    pub field_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateContactDto {
    pub given_name:      Option<String>,
    pub middle_name:     Option<String>,
    pub family_name:     Option<String>,
    pub name_prefix:     Option<String>,
    pub name_suffix:     Option<String>,
    pub nickname:        Option<String>,
    pub display_name:    Option<String>,
    pub organization:    Option<String>,
    pub department:      Option<String>,
    pub job_title:       Option<String>,
    pub avatar_color:    Option<String>,
    #[serde(default)]
    pub emails:          Vec<ContactField>,
    #[serde(default)]
    pub phones:          Vec<ContactField>,
    #[serde(default)]
    pub addresses:       Vec<AddressField>,
    #[serde(default)]
    pub urls:            Vec<ContactField>,
    #[serde(default)]
    pub dates:           Vec<DateField>,
    #[serde(default)]
    pub relations:       Vec<ContactField>,
    #[serde(default)]
    pub instant_messages: Vec<ContactField>,
    #[serde(default)]
    pub custom_fields:   Vec<CustomField>,
    pub notes:           Option<String>,
    pub is_starred:      Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateContactDto {
    pub given_name:      Option<String>,
    pub middle_name:     Option<String>,
    pub family_name:     Option<String>,
    pub name_prefix:     Option<String>,
    pub name_suffix:     Option<String>,
    pub nickname:        Option<String>,
    pub display_name:    Option<String>,
    pub organization:    Option<String>,
    pub department:      Option<String>,
    pub job_title:       Option<String>,
    pub avatar_color:    Option<String>,
    pub emails:          Option<Vec<ContactField>>,
    pub phones:          Option<Vec<ContactField>>,
    pub addresses:       Option<Vec<AddressField>>,
    pub urls:            Option<Vec<ContactField>>,
    pub dates:           Option<Vec<DateField>>,
    pub relations:       Option<Vec<ContactField>>,
    pub instant_messages: Option<Vec<ContactField>>,
    pub custom_fields:   Option<Vec<CustomField>>,
    pub notes:           Option<String>,
    pub is_starred:      Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListContactsParams {
    pub q:          Option<String>,
    pub group_id:   Option<Uuid>,
    pub starred:    Option<bool>,
    pub trashed:    Option<bool>,
    pub limit:      Option<i64>,
    pub offset:     Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContactsListResponse {
    pub contacts: Vec<Contact>,
    pub total:    i64,
}
