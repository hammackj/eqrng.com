use axum::extract::FromRef;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoteType {
    pub id: Option<i64>,
    pub name: String,
    pub display_name: String,
    pub color_class: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZoneNote {
    pub id: Option<i64>,
    pub zone_id: i64,
    pub note_type_id: i64,
    pub content: String,
    pub note_type: Option<NoteType>,
}

#[derive(Clone)]
pub struct ZoneState {
    pub pool: Arc<SqlitePool>,
}

impl FromRef<crate::AppState> for ZoneState {
    fn from_ref(app_state: &crate::AppState) -> ZoneState {
        app_state.zone_state.clone()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Zone {
    pub id: Option<i64>,
    pub name: String,
    pub level_ranges: Vec<[u8; 2]>,
    pub expansion: String,
    pub continent: String,
    pub zone_type: String,
    pub connections: Vec<String>,
    pub image_url: String,
    pub map_url: String,
    pub rating: u8,
    pub hot_zone: bool,
    pub mission: bool,
    pub notes: Vec<ZoneNote>,
}

#[derive(Deserialize)]
pub struct RangeQuery {
    pub min: Option<u8>,
    pub max: Option<u8>,
    zone_type: Option<String>,
    expansion: Option<String>,
    pub mission: Option<bool>,
    hot_zone: Option<bool>,
    continent: Option<String>,
}

pub async fn random_zone(
    Query(params): Query<RangeQuery>,
    State(state): State<crate::AppState>,
) -> Result<Json<Zone>, StatusCode> {
    let pool = &*state.zone_state.pool;

    let mut query = String::from("SELECT * FROM zones WHERE 1=1");
    let mut bindings: Vec<String> = Vec::new();

    if let Some(ref zone_type) = params.zone_type {
        query.push_str(" AND LOWER(zone_type) = LOWER(?)");
        bindings.push(zone_type.clone());
    }

    if let Some(ref expansion) = params.expansion {
        query.push_str(" AND LOWER(expansion) = LOWER(?)");
        bindings.push(expansion.clone());
    }

    if let Some(ref continent) = params.continent {
        query.push_str(" AND LOWER(continent) = LOWER(?)");
        bindings.push(continent.clone());
    }

    if let Some(mission) = params.mission {
        query.push_str(" AND mission = ?");
        bindings.push(if mission { "1" } else { "0" }.to_string());
    }

    if let Some(hot_zone) = params.hot_zone {
        query.push_str(" AND hot_zone = ?");
        bindings.push(if hot_zone { "1" } else { "0" }.to_string());
    }

    query.push_str(" ORDER BY RANDOM() LIMIT 100");

    let mut sql_query = sqlx::query(&query);
    for binding in &bindings {
        sql_query = sql_query.bind(binding);
    }

    let rows = sql_query.fetch_all(pool).await.map_err(|e| {
        eprintln!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut matching_zones: Vec<Zone> = Vec::new();

    for row in rows {
        let level_ranges_json: String = row.get("level_ranges");
        let connections_json: String = row.get("connections");

        let level_ranges: Vec<[u8; 2]> = serde_json::from_str(&level_ranges_json)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let connections: Vec<String> = serde_json::from_str(&connections_json)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let zone = Zone {
            id: Some(row.get::<i64, _>("id")),
            name: row.get("name"),
            level_ranges: level_ranges.clone(),
            expansion: row.get("expansion"),
            continent: row.get("continent"),
            zone_type: row.get("zone_type"),
            connections,
            image_url: row.get("image_url"),
            map_url: row.get("map_url"),
            rating: row.get::<i32, _>("rating") as u8,
            hot_zone: row.get("hot_zone"),
            mission: row.get("mission"),
            notes: Vec::new(),
        };

        let mut level_match = true;
        if params.min.is_some() || params.max.is_some() {
            level_match = false;
            if let (Some(min), Some(max)) = (params.min, params.max) {
                if level_ranges
                    .iter()
                    .any(|&[lmin, lmax]| lmin <= min && lmax >= max)
                {
                    level_match = true;
                }
            } else if let Some(min) = params.min {
                if level_ranges.iter().any(|&[_lmin, lmax]| lmax >= min) {
                    level_match = true;
                }
            } else if let Some(max) = params.max {
                if level_ranges.iter().any(|&[lmin, _lmax]| lmin <= max) {
                    level_match = true;
                }
            }
        }

        if level_match {
            matching_zones.push(zone);
        }
    }

    if matching_zones.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();

    if let Some(zone) = matching_zones.choose(&mut rng) {
        Ok(Json(zone.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn get_zone_notes_endpoint(
    Path(zone_id): Path<i64>,
    State(state): State<crate::AppState>,
) -> Result<Json<Vec<ZoneNote>>, StatusCode> {
    let pool = &*state.zone_state.pool;

    match get_zone_notes(pool, zone_id).await {
        Ok(notes) => Ok(Json(notes)),
        Err(_) => Ok(Json(Vec::new())), // Return empty array on error
    }
}

pub async fn get_all_zones(pool: &SqlitePool) -> Result<Vec<Zone>, sqlx::Error> {
    let rows = sqlx::query("SELECT * FROM zones ORDER BY name")
        .fetch_all(pool)
        .await?;

    let mut zones = Vec::new();

    for row in rows {
        let level_ranges_json: String = row.get("level_ranges");
        let connections_json: String = row.get("connections");

        let level_ranges: Vec<[u8; 2]> =
            serde_json::from_str(&level_ranges_json).unwrap_or_default();
        let connections: Vec<String> = serde_json::from_str(&connections_json).unwrap_or_default();

        zones.push(Zone {
            id: Some(row.get::<i64, _>("id")),
            name: row.get("name"),
            level_ranges,
            expansion: row.get("expansion"),
            continent: row.get("continent"),
            zone_type: row.get("zone_type"),
            connections,
            image_url: row.get("image_url"),
            map_url: row.get("map_url"),
            rating: row.get::<i32, _>("rating") as u8,
            hot_zone: row.get("hot_zone"),
            mission: row.get("mission"),
            notes: Vec::new(), // Notes not loaded for bulk operations
        });
    }

    Ok(zones)
}

pub async fn get_zone_notes(pool: &SqlitePool, zone_id: i64) -> Result<Vec<ZoneNote>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            zn.id,
            zn.zone_id,
            zn.note_type_id,
            zn.content,
            nt.name as note_type_name,
            nt.display_name as note_type_display_name,
            nt.color_class as note_type_color_class
        FROM zone_notes zn
        JOIN note_types nt ON zn.note_type_id = nt.id
        WHERE zn.zone_id = ?
        ORDER BY zn.created_at ASC
        "#,
    )
    .bind(zone_id)
    .fetch_all(pool)
    .await?;

    let mut notes = Vec::new();

    for row in rows {
        notes.push(ZoneNote {
            id: Some(row.get::<i64, _>("id")),
            zone_id: row.get("zone_id"),
            note_type_id: row.get("note_type_id"),
            content: row.get("content"),
            note_type: Some(NoteType {
                id: Some(row.get("note_type_id")),
                name: row.get("note_type_name"),
                display_name: row.get("note_type_display_name"),
                color_class: row.get("note_type_color_class"),
            }),
        });
    }

    Ok(notes)
}

pub async fn get_note_types(pool: &SqlitePool) -> Result<Vec<NoteType>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, name, display_name, color_class FROM note_types ORDER BY display_name",
    )
    .fetch_all(pool)
    .await?;

    let mut note_types = Vec::new();

    for row in rows {
        note_types.push(NoteType {
            id: Some(row.get::<i64, _>("id")),
            name: row.get("name"),
            display_name: row.get("display_name"),
            color_class: row.get("color_class"),
        });
    }

    Ok(note_types)
}
