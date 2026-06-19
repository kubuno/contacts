use axum::{
    middleware,
    routing::{any, delete, get, patch, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    handlers::{
        bulk, carddav, contacts, directory, events, groups, health, import_export, interactions,
        labels, reminders, settings, shares,
    },
    middleware::require_auth,
    state::AppState,
};

pub fn build(state: AppState) -> Router {
    let authed = Router::new()
        // Contacts CRUD
        .route("/contacts",                   get(contacts::list).post(contacts::create))
        .route("/contacts/bulk",              post(bulk::bulk))
        .route("/contacts/trash",             delete(contacts::empty_trash))
        .route("/contacts/duplicates",        get(contacts::duplicates))
        .route("/contacts/duplicates/merge",  post(contacts::merge))
        .route("/contacts/duplicates/ignore", post(contacts::ignore_duplicate))
        .route("/contacts/birthdays",         get(contacts::birthdays))
        // Interaction-driven views
        .route("/contacts/frequent",          get(interactions::frequent))
        .route("/contacts/recent",            get(interactions::recent))
        .route("/contacts/follow-up",         get(interactions::to_follow_up))
        .route("/contacts/:id",               get(contacts::get).patch(contacts::update))
        .route("/contacts/:id/history",       get(contacts::history))
        .route("/contacts/:id/interactions",  get(interactions::list_for_contact).post(interactions::add))
        .route("/contacts/:id/trash",         post(contacts::trash))
        .route("/contacts/:id/restore",       post(contacts::restore))
        .route("/contacts/:id/delete",        delete(contacts::delete_permanently))
        .route("/contacts/:id/star",          post(contacts::star))
        .route("/contacts/:id/unstar",        post(contacts::unstar))
        .route("/contacts/:id/archive",       post(contacts::archive))
        .route("/contacts/:id/unarchive",     post(contacts::unarchive))
        .route("/contacts/:id/block",         post(contacts::block))
        .route("/contacts/:id/unblock",       post(contacts::unblock))
        .route("/contacts/:id/avatar",        post(contacts::upload_avatar).get(contacts::get_avatar))
        // Labels
        .route("/labels",                     get(labels::list).post(labels::create))
        .route("/labels/:id",                 patch(labels::update).delete(labels::delete))
        .route("/labels/:id/members",         post(labels::add_members).delete(labels::remove_members))
        // Reminders
        .route("/reminders",                  get(reminders::list).post(reminders::create))
        .route("/reminders/:id",              patch(reminders::update).delete(reminders::delete))
        // Shares
        .route("/shares",                     get(shares::list).post(shares::create))
        .route("/shares/:id",                 delete(shares::revoke))
        // Settings, stats, CardDAV token
        .route("/settings",                   get(settings::get).patch(settings::update))
        .route("/stats",                      get(settings::stats))
        .route("/carddav/token",              get(carddav::token_info).post(carddav::generate_token).delete(carddav::revoke_token))
        // Groups
        .route("/groups",                     get(groups::list).post(groups::create))
        .route("/groups/:id",                 patch(groups::update).delete(groups::delete))
        .route("/groups/:id/members",         post(groups::add_members))
        .route("/groups/:id/members/:cid",    delete(groups::remove_member))
        // Import / Export
        .route("/export.vcf",                 get(import_export::export_vcf))
        .route("/export.csv",                 get(import_export::export_csv))
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
        // Public share view (token in path)
        .route("/shared/:token", get(shares::public_view))
        // CardDAV protocol (Basic-auth via the CardDAV token, not platform JWT)
        .route("/dav", any(carddav::handle))
        .route("/dav/", any(carddav::handle))
        .route("/dav/*rest", any(carddav::handle))
        .with_state(state);

    Router::new()
        .merge(public_routes)
        .nest("/", authed)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
