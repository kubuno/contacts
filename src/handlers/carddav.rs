use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use base64::Engine;
use uuid::Uuid;

use crate::{services::carddav_service, services::vcard_service, state::AppState};

const NS: &str = "xmlns:d=\"DAV:\" xmlns:card=\"urn:ietf:params:xml:ns:carddav\"";

/// Single entry point for every CardDAV verb (PROPFIND/REPORT/GET/PUT/DELETE/
/// OPTIONS), routed through axum's `any`. Authentication is HTTP Basic with the
/// CardDAV token as the password. Hrefs are built relative to the request path
/// so the collection resolves correctly whether hit directly or via the proxy.
pub async fn handle(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    dav_dispatch(&state, &method, &headers, &body, uri.path()).await
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Basic realm=\"Kubuno Contacts\"")],
        "Authentification requise",
    )
        .into_response()
}

fn parse_basic(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let b64 = auth.strip_prefix("Basic ")?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    let s = String::from_utf8(decoded).ok()?;
    // username:password — the password carries the token.
    let (_, pass) = s.split_once(':')?;
    Some(pass.to_string())
}

async fn dav_dispatch(
    state: &AppState,
    method: &Method,
    headers: &HeaderMap,
    body: &Bytes,
    req_path: &str,
) -> Response {
    // OPTIONS is unauthenticated (capability discovery).
    if method.as_str() == "OPTIONS" {
        return (
            StatusCode::OK,
            [
                ("DAV", "1, 2, 3, addressbook"),
                ("Allow", "OPTIONS, GET, PUT, DELETE, PROPFIND, REPORT"),
            ],
            "",
        )
            .into_response();
    }

    let token = match parse_basic(headers) {
        Some(t) => t,
        None => return unauthorized(),
    };
    let owner = match carddav_service::owner_for_token(&state.db, &token).await {
        Ok(Some(o)) => o,
        _ => return unauthorized(),
    };

    // Path segments after the "/dav" prefix.
    let after = req_path.split("/dav").nth(1).unwrap_or(req_path);
    let segs: Vec<&str> = after.split('/').filter(|s| !s.is_empty()).collect();

    // Base href prefix as the client addressed it (keep trailing context).
    let base = if let Some(idx) = req_path.find("/dav") {
        &req_path[..idx + 4]
    } else {
        "/dav"
    };
    let home = format!("{base}/{owner}/");
    let collection = format!("{base}/{owner}/default/");

    match method.as_str() {
        "PROPFIND" => {
            let depth = headers.get("depth").and_then(|v| v.to_str().ok()).unwrap_or("0");
            propfind(state, owner, &segs, &home, &collection, depth).await
        }
        "REPORT" => report(state, owner, &collection, body).await,
        "GET" | "HEAD" => {
            if let Some(uid) = uid_from_segs(&segs) {
                match carddav_service::get_by_uid(&state.db, owner, &uid).await {
                    Ok(Some(c)) => (
                        StatusCode::OK,
                        [
                            (header::CONTENT_TYPE, "text/vcard; charset=utf-8".to_string()),
                            (header::ETAG, format!("\"{}\"", c.etag)),
                        ],
                        vcard_service::contact_to_vcard(&c),
                    )
                        .into_response(),
                    _ => StatusCode::NOT_FOUND.into_response(),
                }
            } else {
                StatusCode::NOT_FOUND.into_response()
            }
        }
        "PUT" => {
            let Some(uid) = uid_from_segs(&segs) else {
                return StatusCode::BAD_REQUEST.into_response();
            };
            let vcf = String::from_utf8_lossy(body);
            match carddav_service::put_vcard(&state.db, owner, &uid, &vcf).await {
                Ok(etag) => (
                    StatusCode::CREATED,
                    [(header::ETAG, format!("\"{etag}\""))],
                    "",
                )
                    .into_response(),
                Err(e) => e.into_response(),
            }
        }
        "DELETE" => {
            let Some(uid) = uid_from_segs(&segs) else {
                return StatusCode::BAD_REQUEST.into_response();
            };
            match carddav_service::delete_by_uid(&state.db, owner, &uid).await {
                Ok(true) => StatusCode::NO_CONTENT.into_response(),
                Ok(false) => StatusCode::NOT_FOUND.into_response(),
                Err(e) => e.into_response(),
            }
        }
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

fn uid_from_segs(segs: &[&str]) -> Option<String> {
    segs.last()
        .filter(|s| s.ends_with(".vcf"))
        .map(|s| s.trim_end_matches(".vcf").to_string())
}

fn multistatus(body: String) -> Response {
    (
        StatusCode::MULTI_STATUS,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        format!("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n{body}"),
    )
        .into_response()
}

async fn propfind(
    state: &AppState,
    owner: Uuid,
    segs: &[&str],
    home: &str,
    collection: &str,
    depth: &str,
) -> Response {
    let is_collection = segs.last().map(|s| *s == "default").unwrap_or(false);

    if !is_collection {
        // Principal / home-set discovery.
        let body = format!(
            "<d:multistatus {NS}>\
               <d:response>\
                 <d:href>{home}</d:href>\
                 <d:propstat><d:prop>\
                   <d:current-user-principal><d:href>{home}</d:href></d:current-user-principal>\
                   <d:principal-URL><d:href>{home}</d:href></d:principal-URL>\
                   <card:addressbook-home-set><d:href>{home}</d:href></card:addressbook-home-set>\
                   <d:resourcetype><d:collection/><d:principal/></d:resourcetype>\
                 </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>\
               </d:response>\
               <d:response>\
                 <d:href>{collection}</d:href>\
                 <d:propstat><d:prop>\
                   <d:resourcetype><d:collection/><card:addressbook/></d:resourcetype>\
                   <d:displayname>Contacts</d:displayname>\
                 </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>\
               </d:response>\
             </d:multistatus>"
        );
        return multistatus(body);
    }

    // Collection properties (+ members when Depth: 1).
    let ctag = carddav_service::ctag(&state.db, owner).await.unwrap_or_default();
    let mut body = format!(
        "<d:multistatus {NS}>\
           <d:response>\
             <d:href>{collection}</d:href>\
             <d:propstat><d:prop>\
               <d:resourcetype><d:collection/><card:addressbook/></d:resourcetype>\
               <d:displayname>Contacts</d:displayname>\
               <cs:getctag xmlns:cs=\"http://calendarserver.org/ns/\">{ctag}</cs:getctag>\
               <card:supported-address-data><card:address-data-type content-type=\"text/vcard\" version=\"3.0\"/></card:supported-address-data>\
             </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>\
           </d:response>"
    );

    if depth != "0" {
        if let Ok(refs) = carddav_service::list_refs(&state.db, owner).await {
            for (uid, etag) in refs {
                body.push_str(&format!(
                    "<d:response>\
                       <d:href>{collection}{uid}.vcf</d:href>\
                       <d:propstat><d:prop>\
                         <d:getetag>\"{etag}\"</d:getetag>\
                         <d:getcontenttype>text/vcard</d:getcontenttype>\
                       </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>\
                     </d:response>"
                ));
            }
        }
    }
    body.push_str("</d:multistatus>");
    multistatus(body)
}

async fn report(state: &AppState, owner: Uuid, collection: &str, body: &Bytes) -> Response {
    let req = String::from_utf8_lossy(body);
    // addressbook-multiget carries explicit <d:href> entries; otherwise
    // (addressbook-query / sync) we return the full set.
    let requested_uids: Vec<String> = req
        .match_indices("<d:href>")
        .filter_map(|(i, _)| {
            let rest = &req[i + 8..];
            rest.find("</d:href>").map(|end| rest[..end].to_string())
        })
        .filter_map(|href| {
            href.rsplit('/').next().and_then(|f| f.strip_suffix(".vcf")).map(|s| s.to_string())
        })
        .collect();

    let mut out = format!("<d:multistatus {NS}>");
    let push = |out: &mut String, uid: &str, etag: &str, vcard: &str| {
        let escaped = xml_escape(vcard);
        out.push_str(&format!(
            "<d:response><d:href>{collection}{uid}.vcf</d:href>\
               <d:propstat><d:prop>\
                 <d:getetag>\"{etag}\"</d:getetag>\
                 <card:address-data>{escaped}</card:address-data>\
               </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>\
             </d:response>"
        ));
    };

    if requested_uids.is_empty() {
        if let Ok(refs) = carddav_service::list_refs(&state.db, owner).await {
            for (uid, etag) in refs {
                if let Ok(Some(c)) = carddav_service::get_by_uid(&state.db, owner, &uid).await {
                    push(&mut out, &uid, &etag, &vcard_service::contact_to_vcard(&c));
                }
            }
        }
    } else {
        for uid in requested_uids {
            if let Ok(Some(c)) = carddav_service::get_by_uid(&state.db, owner, &uid).await {
                push(&mut out, &uid, &c.etag, &vcard_service::contact_to_vcard(&c));
            }
        }
    }
    out.push_str("</d:multistatus>");
    multistatus(out)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ─── Token management (authenticated platform routes) ───────────────────────

use crate::{errors::Result, middleware::ContactsUser};
use axum::{Extension, Json};
use serde_json::{json, Value};

pub async fn token_info(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    let exists = carddav_service::has_token(&state.db, user.id).await?;
    Ok(Json(json!({ "configured": exists, "username": user.email, "path": "/dav" })))
}

pub async fn generate_token(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    let token = carddav_service::regenerate_token(&state.db, user.id).await?;
    Ok(Json(json!({
        "token":    token,
        "username": user.email,
        "url":      format!("/dav/{}/default/", user.id),
    })))
}

pub async fn revoke_token(
    State(state): State<AppState>,
    Extension(user): Extension<ContactsUser>,
) -> Result<Json<Value>> {
    carddav_service::revoke_token(&state.db, user.id).await?;
    Ok(Json(json!({ "ok": true })))
}
