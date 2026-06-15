use axum::{
    middleware,
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    handlers::{contacts, directory, events, groups, health, import_export},
    middleware::require_auth,
    state::AppState,
};

pub fn build(state: AppState) -> Router {
    let authed = Router::new()
        // Contacts CRUD
        .route("/contacts",                   get(contacts::list).post(contacts::create))
        .route("/contacts/trash",             delete(contacts::empty_trash))
        .route("/contacts/duplicates",        get(contacts::duplicates))
        .route("/contacts/:id",               get(contacts::get).patch(contacts::update))
        .route("/contacts/:id/trash",         post(contacts::trash))
        .route("/contacts/:id/restore",       post(contacts::restore))
        .route("/contacts/:id/delete",        delete(contacts::delete_permanently))
        .route("/contacts/:id/star",          post(contacts::star))
        .route("/contacts/:id/unstar",        post(contacts::unstar))
        .route("/contacts/:id/avatar",        post(contacts::upload_avatar).get(contacts::get_avatar))
        // Groups
        .route("/groups",                     get(groups::list).post(groups::create))
        .route("/groups/:id",                 patch(groups::update).delete(groups::delete))
        .route("/groups/:id/members",         post(groups::add_members))
        .route("/groups/:id/members/:cid",    delete(groups::remove_member))
        // Import / Export
        .route("/export.vcf",                 get(import_export::export_vcf))
        .route("/import/vcf",                 post(import_export::import_vcf))
        .route("/import/csv",                 post(import_export::import_csv))
        // Annuaire
        .route("/directory",                  get(directory::search))
        .route("/directory/:user_id/add",     post(directory::add_to_contacts))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state.clone());

    let public_routes = Router::new()
        .route("/health", get(health::health))
        .route("/events", post(events::handle_event))
        .with_state(state);

    Router::new()
        .merge(public_routes)
        .nest("/", authed)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
