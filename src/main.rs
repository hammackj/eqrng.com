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
    let data = fs::read_to_string("zones.json").expect("zones.json missing");
    let classic = fs::read_to_string("zones/classic.json").expect("zones/classic.json missing");
    let kunark = fs::read_to_string("zones/kunark.json").expect("zones/kunark.json missing");
    let velious = fs::read_to_string("zones/velious.json").expect("zones/velious.json missing");
    let sol = fs::read_to_string("zones/shadows_of_luclin.json")
        .expect("zones/shadows_of_luclin.json missing");
    let pop = fs::read_to_string("zones/planes_of_power.json")
        .expect("zones/planes_of_power.json missing");

    let tss = fs::read_to_string("zones/tss.json")
        .expect("zones/tss.json missing");

    let mut zones: Vec<Zone> = serde_json::from_str(&data).expect("Invalid JSON");
    let classic_zones: Vec<Zone> = serde_json::from_str(&classic).expect("Invalid JSON");
    let kunark_zones: Vec<Zone> = serde_json::from_str(&kunark).expect("Invalid JSON");
    let velious_zones: Vec<Zone> = serde_json::from_str(&velious).expect("Invalid JSON");
    let sol_zones: Vec<Zone> = serde_json::from_str(&sol).expect("Invalid JSON");
    let pop_zones: Vec<Zone> = serde_json::from_str(&pop).expect("Invalid JSON");
    let tss_zones: Vec<Zone> = serde_json::from_str(&tss).expect("Invalid JSON");

    zones.extend(classic_zones);
    zones.extend(kunark_zones);
    zones.extend(velious_zones);
    zones.extend(sol_zones);
    zones.extend(pop_zones);
    zones.extend(tss_zones);

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
    let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
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

            z.level_ranges.iter().any(|&[lmin, lmax]| {
                lmin <= params.min && lmax >= params.max
            })
        })
        .cloned()
        .collect();

    if let Some(zone) = matches.choose(&mut rng) {
        Ok(Json(zone.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
