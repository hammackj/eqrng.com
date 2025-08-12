use axum::http::HeaderValue;
use axum::response::Response;
use axum::{Router, http::Method, middleware, routing::get, serve};
use clap::Parser;
use std::env;
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
async fn main() {
    let args = Args::parse();
    // Initialize database
    let pool = eq_rng::setup_database().await.map_err(|e| {
        eprintln!("Failed to initialize database: {}", e);
        std::process::exit(1);
    })?;

    // Check database health
    eq_rng::database_health_check(&pool).await.map_err(|e| {
        eprintln!("Database health check failed: {}", e);
        std::process::exit(1);
    })?;

    let zone_count = eq_rng::get_zones_count(&pool).await.map_err(|e| {
        eprintln!("Failed to get zone count: {}", e);
        std::process::exit(1);
    })?;

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
            // Configure CORS based on environment
            let cors_layer = if env::var("EQ_RNG_ENV").unwrap_or_default() == "development" {
                // Development: Allow localhost origins
                CorsLayer::new()
                    .allow_origin([
                        "http://localhost:3000".parse().map_err(|e| {
                            eprintln!("Invalid CORS origin: http://localhost:3000 - {}", e);
                            std::process::exit(1);
                        })?,
                        "http://localhost:5173".parse().map_err(|e| {
                            eprintln!("Invalid CORS origin: http://localhost:5173 - {}", e);
                            std::process::exit(1);
                        })?, // Vite dev server
                        "http://127.0.0.1:3000".parse().map_err(|e| {
                            eprintln!("Invalid CORS origin: http://127.0.0.1:3000 - {}", e);
                            std::process::exit(1);
                        })?,
                        "http://127.0.0.1:5173".parse().map_err(|e| {
                            eprintln!("Invalid CORS origin: http://127.0.0.1:5173 - {}", e);
                            std::process::exit(1);
                        })?,
                    ])
                    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                    .allow_headers(Any)
                    .allow_credentials(false)
            } else {
                // Production: Restrict to specific domain or same-origin only
                let allowed_origins = env::var("ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "https://yourdomain.com".to_string())
                    .split(',')
                    .filter_map(|origin| origin.trim().parse().ok())
                    .collect::<Vec<_>>();

                if allowed_origins.is_empty() {
                    // Fallback to restrictive CORS if no valid origins
                    CorsLayer::new()
                        .allow_methods([Method::GET])
                        .allow_headers(Any)
                        .allow_credentials(false)
                } else {
                    CorsLayer::new()
                        .allow_origin(allowed_origins)
                        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                        .allow_headers(Any)
                        .allow_credentials(false)
                }
            };
            cors_layer
        })
        .nest_service("/", ServeDir::new("dist"));

    let addr: SocketAddr = format!("0.0.0.0:{}", args.port).parse().map_err(|e| {
        eprintln!("Failed to parse address: {}", e);
        std::process::exit(1);
    })?;

    let listener = TcpListener::bind(addr).await.map_err(|e| {
        eprintln!("Failed to bind to address {}: {}", addr, e);
        std::process::exit(1);
    })?;

    let local_addr = listener.local_addr().map_err(|e| {
        eprintln!("Failed to get local address: {}", e);
        std::process::exit(1);
    })?;
    println!("Listening on {}", local_addr);

    serve(listener, app.into_make_service())
        .await
        .map_err(|e| {
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        })?;
}
