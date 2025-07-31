use axum::{Router, http::Method, routing::get, serve};
use clap::Parser;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use eq_rng::admin;
use eq_rng::classes::{self, ClassRaceState};
use eq_rng::instances::{self, InstanceState};
use eq_rng::links::{self};
use eq_rng::ratings::{self};
use eq_rng::zones::{self, ZoneState};
use eq_rng::{AppState, races, version};

#[derive(Parser)]
#[command(name = "eq_rng")]
#[command(about = "EverQuest Random Number Generator API Server")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    // Initialize database
    let pool = eq_rng::setup_database()
        .await
        .expect("Failed to initialize database");

    // Check database health
    eq_rng::database_health_check(&pool)
        .await
        .expect("Database health check failed");

    let zone_count = eq_rng::get_zones_count(&pool)
        .await
        .expect("Failed to get zone count");

    println!("Database ready with {} zones", zone_count);

    let state = AppState {
        zone_state: ZoneState {
            pool: std::sync::Arc::new(pool.clone()),
        },
        instance_state: InstanceState {
            pool: std::sync::Arc::new(pool),
        },
        class_race_state: ClassRaceState {
            class_race_map: classes::load_classes(),
        },
    };

    let app = Router::new()
        .route("/random_zone", get(zones::random_zone))
        .route("/random_instance", get(instances::random_instance))
        .route("/random_race", get(races::random_race))
        .route("/random_class", get(classes::random_class))
        .route("/version", get(version::version))
        .route("/zones/:zone_id/rating", get(ratings::get_zone_rating))
        .route(
            "/zones/:zone_id/rating",
            axum::routing::post(ratings::submit_zone_rating),
        )
        .route("/zones/:zone_id/ratings", get(ratings::get_zone_ratings))
        .route(
            "/api/ratings/:id",
            axum::routing::delete(ratings::delete_rating),
        )
        .route("/zones/:zone_id/notes", get(zones::get_zone_notes_endpoint))
        .route(
            "/instances/:instance_id/notes",
            get(instances::get_instance_notes_endpoint),
        )
        .route("/api/links", get(links::get_links))
        .route("/api/links/by-category", get(links::get_links_by_category))
        .route("/api/links/categories", get(links::get_categories))
        .route("/api/links", axum::routing::post(links::create_link))
        .route("/api/links/:id", get(links::get_link))
        .route("/api/links/:id", axum::routing::put(links::update_link))
        .route("/api/links/:id", axum::routing::delete(links::delete_link))
        .merge(admin::admin_routes())
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::HEAD,
                    Method::OPTIONS,
                ])
                .allow_headers(Any)
                .allow_credentials(false),
        )
        .nest_service("/", ServeDir::new("dist"));

    let addr: SocketAddr = format!("0.0.0.0:{}", args.port).parse().unwrap();
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());

    serve(listener, app.into_make_service()).await.unwrap();
}
