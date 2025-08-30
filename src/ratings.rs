use axum::extract::FromRef;
use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::Json,
};
use blake3;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use tracing::{error, warn};
use std::net::SocketAddr;

use crate::{AppError, AppResult, AppState};

#[derive(Clone)]
pub struct RatingState {
    pub pool: Arc<SqlitePool>,
}

impl FromRef<crate::AppState> for RatingState {
    fn from_ref(app_state: &crate::AppState) -> RatingState {
        RatingState {
            pool: app_state.zone_state.pool.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZoneRating {
    pub id: Option<i64>,
    pub zone_id: i64,
    pub user_ip: String,
    pub rating: u8,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZoneRatingStats {
    pub zone_id: i64,
    pub average_rating: f64,
    pub total_ratings: i64,
    pub user_rating: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitRatingRequest {
    pub rating: u8,
}

fn hash_ip(ip: &str, config: &crate::AppConfig) -> String {
    let hash = blake3::hash(config.security.rating_ip_hash_key.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(hash.as_bytes());
    let h = blake3::keyed_hash(&key, ip.as_bytes());
    h.to_hex().to_string()
}

// Get rating statistics for a zone
pub async fn get_zone_rating(
    Path(zone_id): Path<i64>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> AppResult<Json<ZoneRatingStats>> {
    let pool = &*state.zone_state.pool;

    // First, verify the zone exists
    let zone_exists = sqlx::query("SELECT id FROM zones WHERE id = ?")
        .bind(zone_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            error!(
                "Database error checking zone existence for zone {}: {}",
                zone_id, e
            );
            AppError::Database(e)
        })?;

    if zone_exists.is_none() {
        return Err(AppError::ZoneNotFound(zone_id));
    }

    // Get rating statistics
    let stats = sqlx::query(
        r#"
        SELECT
            COUNT(*) as total_ratings,
            AVG(CAST(rating AS FLOAT)) as average_rating
        FROM zone_ratings
        WHERE zone_id = ?
        "#,
    )
    .bind(zone_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!(
            "Database error getting rating stats for zone {}: {}",
            zone_id, e
        );
        AppError::Database(e)
    })?;

    let total_ratings: i64 = stats.get("total_ratings");
    let average_rating: Option<f64> = stats.get("average_rating");

    let user_ip = addr.ip().to_string();
    let hashed_ip = hash_ip(&user_ip, &state.config);
    let user_rating = sqlx::query("SELECT rating FROM zone_ratings WHERE zone_id = ? AND user_ip = ?")
        .bind(zone_id)
        .bind(&hashed_ip)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            error!(
                "Database error getting user rating for zone {}: {}",
                zone_id, e
            );
            AppError::Database(e)
        })?
        .map(|row| row.get::<i32, _>("rating") as u8);

    Ok(Json(ZoneRatingStats {
        zone_id,
        average_rating: average_rating.unwrap_or(0.0),
        total_ratings,
        user_rating,
    }))
}

// Submit or update a rating for a zone
pub async fn submit_zone_rating(
    Path(zone_id): Path<i64>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(payload): Json<SubmitRatingRequest>,
) -> AppResult<Json<ZoneRatingStats>> {
    let pool = &*state.zone_state.pool;

    // Validate rating (using hardcoded values for now, will be configurable later)
    if payload.rating < 1 || payload.rating > 5 {
        return Err(AppError::InvalidRating(payload.rating, 1, 5));
    }

    let user_ip = addr.ip().to_string();
    let hashed_ip = hash_ip(&user_ip, &state.config);

    // First, verify the zone exists
    let zone_exists = sqlx::query("SELECT id FROM zones WHERE id = ?")
        .bind(zone_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            error!(
                "Database error checking zone existence for rating submission on zone {}: {}",
                zone_id, e
            );
            AppError::Database(e)
        })?;

    if zone_exists.is_none() {
        return Err(AppError::ZoneNotFound(zone_id));
    }

    // Insert or update the rating
    sqlx::query(
        r#"
        INSERT INTO zone_ratings (zone_id, user_ip, rating, updated_at)
        VALUES (?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(zone_id, user_ip)
        DO UPDATE SET
            rating = excluded.rating,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(zone_id)
    .bind(&hashed_ip)
    .bind(payload.rating as i32)
    .execute(pool)
    .await
    .map_err(|e| {
        error!(
            "Database error submitting rating for zone {}: {}",
            zone_id, e
        );
        AppError::Database(e)
    })?;

    // Write SQL transaction to log file (only logs hashed IP)
    let sql_statement = format!(
        "INSERT INTO zone_ratings (zone_id, user_ip, rating, updated_at) VALUES ({}, '{}', {}, CURRENT_TIMESTAMP) ON CONFLICT(zone_id, user_ip) DO UPDATE SET rating = excluded.rating, updated_at = CURRENT_TIMESTAMP; -- {}\n",
        zone_id,
        hashed_ip.replace("'", "''"), // Escape single quotes
        payload.rating,
        chrono::Utc::now().to_rfc3339()
    );

    if let Err(e) = write_to_transaction_log(&sql_statement) {
        tracing::warn!(error = %e, "Warning: Failed to write to transaction log");
        // Don't fail the request if logging fails
    }

    // Return updated statistics
    get_zone_rating(Path(zone_id), ConnectInfo(addr), State(state)).await
}

// Get all ratings for a zone (admin/debug endpoint)
pub async fn get_zone_ratings(
    Path(zone_id): Path<i64>,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ZoneRating>>> {
    let pool = &*state.zone_state.pool;

    let rows = sqlx::query(
        r#"
        SELECT id, zone_id, user_ip, rating, created_at, updated_at
        FROM zone_ratings
        WHERE zone_id = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(zone_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        error!(
            "Database error getting zone ratings for zone {}: {}",
            zone_id, e
        );
        AppError::Database(e)
    })?;

    let mut ratings = Vec::new();
    for row in rows {
        ratings.push(ZoneRating {
            id: Some(row.get::<i64, _>("id")),
            zone_id: row.get::<i64, _>("zone_id"),
            user_ip: row.get("user_ip"),
            rating: row.get::<i32, _>("rating") as u8,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        });
    }

    Ok(Json(ratings))
}

// Delete a rating by ID (admin/API endpoint)
pub async fn delete_rating(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> AppResult<StatusCode> {
    let pool = &*state.zone_state.pool;

    // First, get the rating details before deletion for transaction logging
    let rating_details =
        sqlx::query("SELECT zone_id, user_ip, rating FROM zone_ratings WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                error!(
                    "Database error getting rating details for deletion (id {}): {}",
                    id, e
                );
                AppError::Database(e)
            })?;

    if let Some(details) = rating_details {
        let zone_id = details.get::<i64, _>("zone_id");
        let user_ip = details.get::<String, _>("user_ip");
        let _old_rating = details.get::<i32, _>("rating") as u8;

        // Delete the rating
        let _ = sqlx::query("DELETE FROM zone_ratings WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("Database error deleting rating (id {}): {}", id, e);
                AppError::Database(e)
            })?;

        // Write SQL transaction to log file
        let sql_statement = format!(
            "DELETE FROM zone_ratings WHERE zone_id = {} AND user_ip = '{}'; -- {} - Deleted via admin\n",
            zone_id,
            user_ip.replace("'", "''"), // Escape single quotes
            chrono::Utc::now().to_rfc3339()
        );

        if let Err(e) = write_to_transaction_log(&sql_statement) {
            warn!("Warning: Failed to write to transaction log: {}", e);
            // Don't fail the request if logging fails
        }

        // Force WAL checkpoint to immediately update main database file
        let _ = crate::checkpoint_wal(pool).await;

        Ok(StatusCode::OK)
    } else {
        Err(AppError::RatingNotFound(id as i64))
    }
}

// Helper function to write SQL statements to transaction log file
fn write_to_transaction_log(sql_statement: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("data/rating_transaction.log")?;

    file.write_all(sql_statement.as_bytes())?;
    file.flush()?;
    Ok(())
}
