use axum::extract::FromRef;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{fs, sync::Arc};

#[derive(Clone)]
pub struct ZoneState {
    pub zones: Arc<Vec<Zone>>,
}

impl FromRef<crate::AppState> for ZoneState {
    fn from_ref(app_state: &crate::AppState) -> ZoneState {
        app_state.zone_state.clone()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Zone {
    name: String,
    level_ranges: Vec<[u8; 2]>,
    expansion: String,
    continent: String,
    zone_type: String,
    connections: Vec<String>,
    image_url: String,
    map_url: String,
    hot_zone: bool,
}

#[derive(Deserialize)]
pub struct RangeQuery {
    min: u8,
    max: u8,
    zone_type: Option<String>,
}

pub fn load_zones() -> Arc<Vec<Zone>> {
    let zone_files = [
        "data/zones/classic.json",
        "data/zones/kunark.json",
        "data/zones/velious.json",
        "data/zones/shadows_of_luclin.json",
        "data/zones/planes_of_power.json",
        "data/zones/loy.json",
        "data/zones/ldon.json",
        "data/zones/god.json",
        "data/zones/oow.json",
        "data/zones/don.json",
        "data/zones/tss.json",
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

pub async fn random_zone(
    Query(params): Query<RangeQuery>,
    State(state): State<crate::AppState>,
) -> Result<Json<Zone>, StatusCode> {
    let mut rng = rand::thread_rng();

    let zones = state.zone_state.zones.clone();

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
