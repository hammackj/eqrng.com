use axum::{Router, routing::get, serve};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

use crate::classes::ClassRaceState;
use crate::zones::ZoneState;

mod classes;
mod races;
mod version;
mod zones;

#[derive(Clone)]
struct AppState {
    zone_state: zones::ZoneState,
    class_race_state: classes::ClassRaceState,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        zone_state: ZoneState {
            zones: zones::load_zones(),
        },
        class_race_state: ClassRaceState {
            class_race_map: classes::load_classes(),
        },
    };

    let app = Router::new()
        .route("/random_zone", get(zones::random_zone))
        .route("/random_race", get(races::random_race))
        .route("/random_class", get(classes::random_class))
        .route("/version", get(version::version))
        .nest_service("/", ServeDir::new("public"))
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:3000".parse().unwrap();
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());

    serve(listener, app.into_make_service()).await.unwrap();
}
