use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    serve,
};

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{fs, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Zone {
    name: String,
    level_ranges: Vec<[u8; 2]>,
    expansion: String,
    continent: String,
    zone_type: String,
    connections: Vec<String>,
    image_url: String,
    map_url: String,
}

#[derive(Deserialize)]
struct RangeQuery {
    min: u8,
    max: u8,
    zone_type: Option<String>,
}

fn load_zones() -> Arc<Vec<Zone>> {
    let zone_files = [
        "zones/classic.json",
        "zones/kunark.json",
        "zones/velious.json",
        "zones/shadows_of_luclin.json",
        "zones/planes_of_power.json",
        "zones/loy.json",
        "zones/tss.json",
    ];

    let mut zones: Vec<Zone> = Vec::new();

    for file in zone_files {
        let content = fs::read_to_string(file).expect(format!("{} missing", file).as_str());
        let zone_data: Vec<Zone> = serde_json::from_str(&content).expect("Invalid JSON");
        zones.extend(zone_data);
    }

    let shared_zones = Arc::new(zones);

    return shared_zones;
}

#[tokio::main]
async fn main() {
    // Load all the zone jsom and merge them together
    // TODO: Move to sqlite once all data is entered and validated
    let zones = load_zones();

    let app = Router::new()
        .route("/random_zone", get(random_zone))
        .nest_service("/", ServeDir::new("public"))
        .with_state(zones);

    // bind via TcpListener
    // TODO: Add CLI Options for Host / Port
    let addr: SocketAddr = "0.0.0.0:3000".parse().unwrap();
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());

    serve(listener, app.into_make_service()).await.unwrap();
}

async fn random_zone(
    Query(params): Query<RangeQuery>,
    State(zones): State<Arc<Vec<Zone>>>,
) -> Result<Json<Zone>, StatusCode> {
    let mut rng = rand::thread_rng();

    let matches: Vec<Zone> = zones
        .iter()
        .filter(|z| {
            if let Some(ref t) = params.zone_type {
                if !z.zone_type.eq_ignore_ascii_case(t) {
                    return false;
                }
            }

            z.level_ranges
                .iter()
                .any(|&[lmin, lmax]| lmin <= params.min && lmax >= params.max)
        })
        .cloned()
        .collect();

    if let Some(zone) = matches.choose(&mut rng) {
        Ok(Json(zone.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
