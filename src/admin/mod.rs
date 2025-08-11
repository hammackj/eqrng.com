use axum::Router;

#[cfg(feature = "admin")]
use axum::extract::State;
#[cfg(feature = "admin")]
use axum::http::StatusCode;
#[cfg(feature = "admin")]
use axum::middleware;

#[cfg(not(feature = "admin"))]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::AppState;

// Module declarations
#[cfg(feature = "admin")]
pub mod dashboard;
#[cfg(feature = "admin")]
pub mod flags;
#[cfg(feature = "admin")]
pub mod instances;
#[cfg(feature = "admin")]
pub mod links;
#[cfg(feature = "admin")]
pub mod notes;
#[cfg(feature = "admin")]
pub mod ratings;
#[cfg(feature = "admin")]
pub mod types;
#[cfg(feature = "admin")]
pub mod utils;
#[cfg(feature = "admin")]
pub mod zones;

// Re-export types for backward compatibility
#[cfg(feature = "admin")]
pub use types::*;

// Re-export main functions
#[cfg(feature = "admin")]
pub use dashboard::{admin_dashboard, log_admin_requests};
#[cfg(feature = "admin")]
pub use flags::*;
#[cfg(feature = "admin")]
pub use instances::*;
#[cfg(feature = "admin")]
pub use links::*;
#[cfg(feature = "admin")]
pub use notes::*;
#[cfg(feature = "admin")]
pub use ratings::*;
#[cfg(feature = "admin")]
pub use utils::*;
#[cfg(feature = "admin")]
pub use zones::*;

#[cfg(feature = "admin")]
async fn trigger_migrations(State(state): State<AppState>) -> Result<String, StatusCode> {
    let pool = &state.zone_state.pool;

    match crate::run_migrations(pool.as_ref()).await {
        Ok(_) => Ok("Migrations completed successfully".to_string()),
        Err(e) => {
            eprintln!("Migration error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(feature = "admin")]
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/admin", axum::routing::get(admin_dashboard))
        .route("/admin/zones", axum::routing::get(list_zones))
        .route("/admin/zones/new", axum::routing::get(new_zone_form))
        .route("/admin/zones", axum::routing::post(create_zone))
        .route("/admin/zones/:id", axum::routing::get(edit_zone_form))
        .route(
            "/admin/zones/:id",
            axum::routing::post(handle_zone_update_or_delete),
        )
        .route("/admin/zones/:id/delete", axum::routing::post(delete_zone))
        .route(
            "/admin/zones/:id/move-to-instances",
            axum::routing::post(move_zone_to_instances),
        )
        .route("/admin/zones/:id/ratings", axum::routing::get(zone_ratings))
        .route("/admin/zones/:id/notes", axum::routing::get(zone_notes))
        .route(
            "/admin/zones/:id/notes",
            axum::routing::post(create_zone_note),
        )
        .route(
            "/admin/zones/:id/notes/:note_id/delete",
            axum::routing::post(delete_zone_note),
        )
        .route(
            "/admin/zones/:id/flags",
            axum::routing::post(create_zone_flag),
        )
        .route(
            "/admin/zones/:id/flags/:flag_id/delete",
            axum::routing::post(delete_zone_flag),
        )
        .route(
            "/admin/zones/:id/flags/:flag_id/delete",
            axum::routing::get(delete_zone_flag),
        )
        .route(
            "/admin/zones/:id/remove-flag/:flag_id",
            axum::routing::get(delete_zone_flag_simple),
        )
        .layer(middleware::from_fn(log_admin_requests))
        .route("/admin/instances", axum::routing::get(list_instances))
        .route(
            "/admin/instances/:id",
            axum::routing::get(edit_instance_form),
        )
        .route(
            "/admin/instances/:id",
            axum::routing::post(handle_instance_update_or_delete),
        )
        .route(
            "/admin/instances/:id/delete",
            axum::routing::post(delete_instance),
        )
        .route(
            "/admin/instances/:id/notes",
            axum::routing::get(instance_notes),
        )
        .route(
            "/admin/instances/:id/notes",
            axum::routing::post(create_instance_note),
        )
        .route(
            "/admin/instances/:id/notes/:note_id/delete",
            axum::routing::post(delete_instance_note),
        )
        .route("/admin/note-types", axum::routing::get(list_note_types))
        .route("/admin/note-types", axum::routing::post(create_note_type))
        .route(
            "/admin/note-types/:id/delete",
            axum::routing::post(delete_note_type),
        )
        .route("/admin/flag-types", axum::routing::get(list_flag_types))
        .route("/admin/flag-types", axum::routing::post(create_flag_type))
        .route(
            "/admin/flag-types/:id",
            axum::routing::get(edit_flag_type_form),
        )
        .route(
            "/admin/flag-types/:id",
            axum::routing::post(update_flag_type),
        )
        .route(
            "/admin/flag-types/:id/delete",
            axum::routing::post(delete_flag_type),
        )
        .route("/admin/ratings", axum::routing::get(list_all_ratings))
        .route(
            "/admin/ratings/:id/delete",
            axum::routing::post(handle_rating_delete),
        )
        .route("/admin/links", axum::routing::get(list_links))
        .route("/admin/links/new", axum::routing::get(new_link_form))
        .route("/admin/links", axum::routing::post(create_link_admin))
        .route("/admin/links/:id", axum::routing::get(edit_link_form))
        .route(
            "/admin/links/:id",
            axum::routing::post(handle_link_update_or_delete),
        )
        .route(
            "/admin/links/:id/delete",
            axum::routing::post(delete_link_admin),
        )
        .route(
            "/admin/dump-database",
            axum::routing::post(dump_database_sql),
        )
        .route("/admin/migrate", axum::routing::post(trigger_migrations))
}

#[cfg(not(feature = "admin"))]
pub fn admin_routes() -> Router<AppState> {
    Router::new()
}
