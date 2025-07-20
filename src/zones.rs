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
    rating: u8,
    hot_zone: bool,
    mission: bool,
}

#[derive(Deserialize)]
pub struct RangeQuery {
    pub min: Option<u8>,
    pub max: Option<u8>,
    zone_type: Option<String>,
    expansion: Option<String>,
    pub mission: Option<bool>,
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
        "data/zones/god_missions.json",
        "data/zones/oow.json",
        "data/zones/oow_missions.json",
        "data/zones/don.json",
        "data/zones/don_missions.json",
        "data/zones/dodh.json",
        "data/zones/dodh_missions.json",
        "data/zones/ro.json",
        "data/zones/ro_missions.json",
        "data/zones/tss.json",
        "data/zones/tbs.json",
        "data/zones/sof.json",
        "data/zones/sof_missions.json",
        "data/zones/sod.json",
        "data/zones/sod_missions.json",
        "data/zones/uf.json",
        "data/zones/hot.json",
        "data/zones/hot_missions.json",
        "data/zones/tov.json",
        "data/zones/cov.json",
        "data/zones/tol.json",
        "data/zones/nos.json",
        "data/zones/ls.json",
        "data/zones/ls_missions.json",
        "data/zones/tob.json",
        "data/zones/tob_missions.json",
    ];

    let mut zones: Vec<Zone> = Vec::new();

    for file in zone_files {
        let content = fs::read_to_string(file).expect(format!("{} missing", file).as_str());
        let zone_data: Vec<Zone> =
            serde_json::from_str(&content).expect(format!("Invalid JSON File: {}", file).as_str());

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

            // filter by expansion
            if let Some(ref exp) = params.expansion {
                if !z.expansion.eq_ignore_ascii_case(exp) {
                    return false;
                }
            }

            // mission filter
            if let Some(mission_flag) = params.mission {
                if z.mission != mission_flag {
                    return false;
                }
            }

            // level filtering — *only* if at least one bound is set
            if params.min.is_some() || params.max.is_some() {
                // both bounds present → must cover entire interval
                if let (Some(min), Some(max)) = (params.min, params.max) {
                    if !z
                        .level_ranges
                        .iter()
                        .any(|&[lmin, lmax]| lmin <= min && lmax >= max)
                    {
                        return false;
                    }
                }
                // only min present → any range that goes up to at least min
                else if let Some(min) = params.min {
                    if !z.level_ranges.iter().any(|&[_lmin, lmax]| lmax >= min) {
                        return false;
                    }
                }
                // only max present → any range that starts at or below max
                else if let Some(max) = params.max {
                    if !z.level_ranges.iter().any(|&[lmin, _lmax]| lmin <= max) {
                        return false;
                    }
                }
            }
            // if neither min nor max is set, we skip level checks entirely
            true
        })
        .cloned()
        .collect();

    if let Some(zone) = matches.choose(&mut rng) {
        Ok(Json(zone.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
