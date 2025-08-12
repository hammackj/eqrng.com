use axum::http::HeaderValue;
use axum::response::Response;
use axum::{Router, http::Method, middleware, routing::get, serve};
use clap::Parser;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

use eq_rng::{
    AppConfig, AppState, admin, classes, instances, links, races, ratings, version, zones,
};

#[derive(Parser)]
#[command(name = "eq_rng")]
#[command(about = "EverQuest Random Number Generator API Server")]
struct Args {
    /// Port to listen on (overrides config)
    #[arg(short, long)]
    port: Option<u16>,
}

// Security headers middleware
async fn security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();

    // XSS Protection
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );

    // Content Type Options
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );

    // Frame Options
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));

    // Content Security Policy
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static(eq_rng::security::get_csp_header()),
    );

    // Referrer Policy
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Permissions Policy
    headers.insert(
        "Permissions-Policy",
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );

    response
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration first
    let config = AppConfig::load()?;

    // Initialize logging
    eq_rng::logging::init_logging(&config.logging)?;

    info!("Starting EQ RNG server...");

    let args = Args::parse();

    // Use command line port if provided, otherwise use config
    let port = args.port.unwrap_or(config.server.port);

    info!(
        "Server configuration loaded - Port: {}, Environment: {}",
        port,
        if config.is_development() {
            "development"
        } else {
            "production"
        }
    );

    // Initialize database
    let pool = eq_rng::setup_database().await.map_err(|e| {
        error!("Failed to initialize database: {}", e);
        e
    })?;

    // Check database health
    eq_rng::database_health_check(&pool).await.map_err(|e| {
        error!("Database health check failed: {}", e);
        e
    })?;

    let zone_count = eq_rng::get_zones_count(&pool).await.map_err(|e| {
        error!("Failed to get zone count: {}", e);
        e
    })?;

    info!("Database ready with {} zones", zone_count);

    let state = AppState {
        zone_state: zones::ZoneState {
            pool: std::sync::Arc::new(pool.clone()),
        },
        instance_state: instances::InstanceState {
            pool: std::sync::Arc::new(pool),
        },
        class_race_state: classes::ClassRaceState {
            class_race_map: classes::load_classes(),
        },
    };

    let app = Router::new()
        .route(
            "/random_zone",
            get(|axum::extract::Query(params): axum::extract::Query<zones::RangeQuery>, axum::extract::State(state): axum::extract::State<crate::AppState>| async move {
                zones::random_zone(axum::extract::Query(params), axum::extract::State(state)).await
            }),
        )
        .route("/random_instance", get(instances::random_instance))
        .route("/random_race", get(races::random_race))
        .route("/random_class", get(classes::random_class))
        .route("/version", get(version::version))
        .route("/flag-types", get(zones::get_flag_types_api))
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
        .layer(middleware::map_response(security_headers))
        .layer({
            // Configure CORS based on configuration
            let cors_origins = config.get_cors_origins();
            let parsed_origins: Result<Vec<_>, _> =
                cors_origins.iter().map(|origin| origin.parse()).collect();

            match parsed_origins {
                Ok(origins) => {
                    info!("CORS configured with {} origins", origins.len());
                    CorsLayer::new()
                        .allow_origin(origins)
                        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                        .allow_headers(Any)
                        .allow_credentials(false)
                }
                Err(e) => {
                    warn!(
                        "Failed to parse CORS origins: {}, using restrictive CORS",
                        e
                    );
                    CorsLayer::new()
                        .allow_methods([Method::GET])
                        .allow_headers(Any)
                        .allow_credentials(false)
                }
            }
        })
        .nest_service("/", ServeDir::new("dist"));

    let addr: SocketAddr = format!("{}:{}", config.server.host, port)
        .parse()
        .map_err(|e| {
            error!(
                "Failed to parse address {}:{}: {}",
                config.server.host, port, e
            );
            e
        })?;

    let listener = TcpListener::bind(addr).await.map_err(|e| {
        error!("Failed to bind to address {}: {}", addr, e);
        e
    })?;

    let local_addr = listener.local_addr().map_err(|e| {
        error!("Failed to get local address: {}", e);
        e
    })?;

    info!("Listening on {}", local_addr);

    serve(listener, app.into_make_service())
        .await
        .map_err(|e| {
            error!("Server error: {}", e);
            e
        })?;

    Ok(())
}
