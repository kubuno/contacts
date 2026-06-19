use axum::{
    extract::{Multipart, Query, State},
    http::header,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    errors::{ContactsError, Result},
    middleware::ContactsUser,
    models::contact::ListContactsParams,
    services::{contact_service, vcard_service},
    state::AppState,
};

#[derive(Deserialize)]
pub struct ExportParams {
    pub group_id: Option<uuid::Uuid>,
    pub starred:  Option<bool>,
}

pub async fn export_vcf(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(params): Query<ExportParams>,
) -> Result<Response> {
    let list_params = ListContactsParams {
        q:        None,
        group_id: params.group_id,
        label_id: None,
        starred:  params.starred,
        trashed:  Some(false),
        archived: None,
        filter:   None,
        sort:     None,
        limit:    Some(10000),
        offset:   Some(0),
    };

    let result = contact_service::list_contacts(&state.db, user.id, &list_params).await?;
    let contacts: Vec<_> = result.contacts.into_iter().map(|c| c.contact).collect();
    let vcf = vcard_service::contacts_to_vcf(&contacts);

    Ok((
        [
            (header::CONTENT_TYPE, "text/vcard; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"contacts.vcf\""),
        ],
        vcf,
    ).into_response())
}

/// Exports contacts as a CSV with columns broadly compatible with Google /
/// Outlook imports.
pub async fn export_csv(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    Query(params): Query<ExportParams>,
) -> Result<Response> {
    let list_params = ListContactsParams {
        q: None, group_id: params.group_id, label_id: None, starred: params.starred,
        trashed: Some(false), archived: None, filter: None, sort: None,
        limit: Some(10000), offset: Some(0),
    };
    let result = contact_service::list_contacts(&state.db, user.id, &list_params).await?;

    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record([
        "First Name", "Last Name", "Display Name", "Nickname", "Organization",
        "Department", "Job Title", "Email", "Phone", "Address", "Notes",
    ])
    .map_err(|e| ContactsError::Internal(e.into()))?;

    for cwl in &result.contacts {
        let c = &cwl.contact;
        let email = c.emails.0.first().map(|e| e.value.clone()).unwrap_or_default();
        let phone = c.phones.0.first().map(|p| p.value.clone()).unwrap_or_default();
        let address = c.addresses.0.first().map(|a| {
            [a.street.as_deref(), a.city.as_deref(), a.postcode.as_deref(), a.country.as_deref()]
                .into_iter().flatten().collect::<Vec<_>>().join(", ")
        }).unwrap_or_default();
        wtr.write_record([
            c.given_name.clone().unwrap_or_default(),
            c.family_name.clone().unwrap_or_default(),
            c.display_name.clone(),
            c.nickname.clone().unwrap_or_default(),
            c.organization.clone().unwrap_or_default(),
            c.department.clone().unwrap_or_default(),
            c.job_title.clone().unwrap_or_default(),
            email,
            phone,
            address,
            c.notes.clone().unwrap_or_default(),
        ])
        .map_err(|e| ContactsError::Internal(e.into()))?;
    }

    let data = wtr.into_inner().map_err(|e| ContactsError::Internal(e.into()))?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"contacts.csv\""),
        ],
        data,
    ).into_response())
}

pub async fn import_vcf(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    mut multipart: Multipart,
) -> Result<Json<Value>> {
    let mut vcf_content = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let bytes = field.bytes().await.map_err(|e| ContactsError::Validation(e.to_string()))?;
            vcf_content = String::from_utf8(bytes.to_vec())
                .map_err(|_| ContactsError::Validation("Encodage du fichier invalide".into()))?;
            break;
        }
    }

    if vcf_content.is_empty() {
        return Err(ContactsError::Validation("Fichier VCF manquant".into()));
    }

    let dtos = vcard_service::parse_vcf(&vcf_content);
    let total = dtos.len();
    let mut imported = 0;
    let mut errors = 0;

    for dto in dtos {
        match contact_service::create_contact(&state.db, user.id, &dto).await {
            Ok(_) => imported += 1,
            Err(e) => {
                tracing::warn!(error = %e, "Import VCF: contact ignoré");
                errors += 1;
            }
        }
    }

    Ok(Json(json!({ "total": total, "imported": imported, "errors": errors })))
}

pub async fn import_csv(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
    mut multipart: Multipart,
) -> Result<Json<Value>> {
    let mut csv_content = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name().unwrap_or("") == "file" {
            let bytes = field.bytes().await.map_err(|e| ContactsError::Validation(e.to_string()))?;
            csv_content = String::from_utf8(bytes.to_vec())
                .map_err(|_| ContactsError::Validation("Encodage CSV invalide".into()))?;
            break;
        }
    }

    if csv_content.is_empty() {
        return Err(ContactsError::Validation("Fichier CSV manquant".into()));
    }

    let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());
    let headers = rdr.headers().map_err(|e| ContactsError::Validation(e.to_string()))?.clone();

    let mut imported = 0;
    let mut errors = 0;

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => { errors += 1; continue; }
        };

        let get = |name: &str| -> Option<String> {
            headers.iter().position(|h| h.to_lowercase() == name.to_lowercase())
                .and_then(|i| record.get(i))
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        };

        let dto = crate::models::contact::CreateContactDto {
            given_name:       get("First Name").or_else(|| get("given_name")),
            family_name:      get("Last Name").or_else(|| get("family_name")),
            middle_name:      get("Middle Name"),
            name_prefix:      get("Title"),
            name_suffix:      None,
            nickname:         get("Nickname"),
            display_name:     get("Name").or_else(|| get("display_name")),
            organization:     get("Organization").or_else(|| get("Company")),
            department:       get("Department"),
            job_title:        get("Job Title").or_else(|| get("Title")),
            avatar_color:     None,
            pronouns:         None,
            emails:           get("Email Address").or_else(|| get("E-mail Address")).or_else(|| get("email"))
                .map(|e| vec![crate::models::contact::ContactField {
                    label: None, value: e, field_type: "work".into()
                }]).unwrap_or_default(),
            phones:           get("Phone Number").or_else(|| get("Mobile Phone")).or_else(|| get("phone"))
                .map(|p| vec![crate::models::contact::ContactField {
                    label: None, value: p, field_type: "mobile".into()
                }]).unwrap_or_default(),
            addresses:        vec![],
            urls:             vec![],
            dates:            vec![],
            relations:        vec![],
            instant_messages: vec![],
            custom_fields:    vec![],
            notes:            get("Notes"),
            is_starred:       None,
        };

        match contact_service::create_contact(&state.db, user.id, &dto).await {
            Ok(_) => imported += 1,
            Err(e) => {
                tracing::warn!(error = %e, "Import CSV: ligne ignorée");
                errors += 1;
            }
        }
    }

    Ok(Json(json!({ "imported": imported, "errors": errors })))
}
