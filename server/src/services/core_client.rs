use serde_json::{json, Value};
use uuid::Uuid;

use crate::config::Settings;

/// Publishes an event to the core's internal bus (best-effort, never fails the
/// caller). The body must match the core `AppEvent` enum
/// (`{ "type": <variant>, "payload": { ... } }`).
pub async fn publish(settings: &Settings, event: Value) {
    let url    = format!("{}/internal/events/publish", settings.core.url);
    let secret = settings.core.internal_secret.clone();
    // Fire-and-forget so request latency never blocks the handler.
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        if let Err(e) = client
            .post(&url)
            .header("X-Internal-Secret", secret)
            .json(&event)
            .send()
            .await
        {
            tracing::warn!(error = %e, "Publication d'event vers le core échouée");
        }
    });
}

/// Builds a `Custom` event delivered over WebSocket only to `recipients`.
pub fn user_notification(recipients: &[Uuid], event_type: &str, mut payload: Value) -> Value {
    if let Value::Object(ref mut map) = payload {
        map.insert(
            "recipient_user_ids".into(),
            json!(recipients.iter().map(|u| u.to_string()).collect::<Vec<_>>()),
        );
    }
    json!({
        "type": "Custom",
        "payload": {
            "event_type": event_type,
            "module_id":  "contacts",
            "payload":    payload,
        }
    })
}

/// Builds a `ContactUpdated` event (recognised natively by the core enum).
pub fn contact_updated(contact_id: Uuid, user_id: Uuid) -> Value {
    json!({
        "type": "ContactUpdated",
        "payload": {
            "contact_id": contact_id,
            "user_id":    user_id,
            "module_id":  "contacts",
        }
    })
}

/// Builds a generic `Custom` event for contact lifecycle (created/deleted),
/// since those variants are not part of the core enum.
pub fn contact_lifecycle(event_type: &str, contact_id: Uuid, user_id: Uuid) -> Value {
    json!({
        "type": "Custom",
        "payload": {
            "event_type": event_type,
            "module_id":  "contacts",
            "payload": {
                "contact_id": contact_id.to_string(),
                "user_id":    user_id.to_string(),
            }
        }
    })
}
