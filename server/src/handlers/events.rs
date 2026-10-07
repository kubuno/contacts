use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    services::other_contact_service::{self as other, SeenInterlocutor},
    state::AppState,
};

#[derive(Deserialize)]
pub struct KubunoEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload:    serde_json::Value,
}

/// Suffix every "someone was dealt with" event ends in, whatever module sends
/// it: `mail.interlocutor`, `chat.interlocutor`, … Matching on the suffix rather
/// than on a list of module names means a new module feeds "Other contacts"
/// without this file changing.
const INTERLOCUTOR_SUFFIX: &str = ".interlocutor";

/// The payload a module puts in its `*.interlocutor` event.
#[derive(Deserialize)]
struct InterlocutorPayload {
    owner_id:      Uuid,
    interlocutors: Vec<ReportedInterlocutor>,
}

#[derive(Deserialize)]
struct ReportedInterlocutor {
    kind:         String,
    value:        String,
    display_name: Option<String>,
}

/// Called by the core when a subscribed event fires.
///
/// Account events (`UserCreated`/`UserUpdated`/`UserDeleted`) are acknowledged
/// without side effects: the module used to mirror them into a local directory
/// table, which bypassed the instance sharing policy, so the directory is read
/// straight from the core's governed endpoints instead.
///
/// `<module>.interlocutor` is what fills "Other contacts". The exchange goes
/// through the CORE's bus rather than a direct call, so mail and chat need to
/// know nothing about this module — they publish that they dealt with someone,
/// and an instance without an address book simply has no subscriber.
pub async fn handle_event(
    State(state): State<AppState>,
    Json(event): Json<KubunoEvent>,
) -> Result<Json<Value>> {
    // Only `Custom` carries a module's own event; its real name is inside.
    if event.event_type != "Custom" {
        return Ok(Json(json!({ "ok": true })));
    }
    let inner_type = event.payload.get("event_type").and_then(Value::as_str).unwrap_or("");
    if !inner_type.ends_with(INTERLOCUTOR_SUFFIX) {
        return Ok(Json(json!({ "ok": true })));
    }
    let module = event.payload.get("module_id").and_then(Value::as_str).unwrap_or("?").to_string();

    let body = match event.payload.get("payload") {
        Some(p) => p.clone(),
        None => return Ok(Json(json!({ "ok": true }))),
    };
    // A malformed payload is dropped, not answered with an error: the core would
    // retry it forever, and no retry can fix a producer's mistake.
    let parsed: InterlocutorPayload = match serde_json::from_value(body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, module = %module, "interlocutors: charge utile illisible, ignorée");
            return Ok(Json(json!({ "ok": true, "recorded": 0 })));
        }
    };

    let seen: Vec<SeenInterlocutor> = parsed
        .interlocutors
        .into_iter()
        .map(|i| SeenInterlocutor {
            owner_id:     parsed.owner_id,
            kind:         i.kind,
            value:        i.value,
            display_name: i.display_name,
        })
        .collect();

    let recorded = other::record_seen(&state.db, &module, &seen).await?;
    Ok(Json(json!({ "ok": true, "recorded": recorded })))
}
