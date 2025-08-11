use axum::extract::FromRef;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;

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

#[derive(Debug, Deserialize)]
pub struct RatingQuery {
    pub user_ip: Option<String>,
}

// Get rating statistics for a zone
pub async fn get_zone_rating(
    Path(zone_id): Path<i64>,
    Query(params): Query<RatingQuery>,
    State(state): State<crate::AppState>,
) -> Result<Json<ZoneRatingStats>, StatusCode> {
    let pool = &*state.zone_state.pool;

    // First, verify the zone exists
    let zone_exists = sqlx::query("SELECT id FROM zones WHERE id = ?")
        .bind(zone_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            eprintln!("Database error checking zone existence: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if zone_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
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
        eprintln!("Database error getting rating stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total_ratings: i64 = stats.get("total_ratings");
    let average_rating: Option<f64> = stats.get("average_rating");

    // Get user's rating if user_ip is provided
    let user_rating = if let Some(ref user_ip) = params.user_ip {
        sqlx::query("SELECT rating FROM zone_ratings WHERE zone_id = ? AND user_ip = ?")
            .bind(zone_id)
            .bind(user_ip)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                eprintln!("Database error getting user rating: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .map(|row| row.get::<i32, _>("rating") as u8)
    } else {
        None
    };

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
    Query(params): Query<RatingQuery>,
    State(state): State<crate::AppState>,
    Json(payload): Json<SubmitRatingRequest>,
) -> Result<Json<ZoneRatingStats>, StatusCode> {
    let pool = &*state.zone_state.pool;

    // Validate rating
    if payload.rating < 1 || payload.rating > 5 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get user IP (in a real app, you'd extract this from the request headers)
    let user_ip = params.user_ip.unwrap_or_else(|| "127.0.0.1".to_string());

    // First, verify the zone exists
    let zone_exists = sqlx::query("SELECT id FROM zones WHERE id = ?")
        .bind(zone_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            eprintln!("Database error checking zone existence: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if zone_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
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
    .bind(&user_ip)
    .bind(payload.rating as i32)
    .execute(pool)
    .await
    .map_err(|e| {
        eprintln!("Database error submitting rating: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Write SQL transaction to log file
    let sql_statement = format!(
        "INSERT INTO zone_ratings (zone_id, user_ip, rating, updated_at) VALUES ({}, '{}', {}, CURRENT_TIMESTAMP) ON CONFLICT(zone_id, user_ip) DO UPDATE SET rating = excluded.rating, updated_at = CURRENT_TIMESTAMP; -- {}\n",
        zone_id,
        user_ip.replace("'", "''"), // Escape single quotes
        payload.rating,
        chrono::Utc::now().to_rfc3339()
    );

    if let Err(e) = write_to_transaction_log(&sql_statement) {
        eprintln!("Warning: Failed to write to transaction log: {}", e);
        // Don't fail the request if logging fails
    }

    // Return updated statistics
    get_zone_rating(
        Path(zone_id),
        Query(RatingQuery {
            user_ip: Some(user_ip),
        }),
        State(state),
    )
    .await
}

// Get all ratings for a zone (admin/debug endpoint)
pub async fn get_zone_ratings(
    Path(zone_id): Path<i64>,
    State(state): State<crate::AppState>,
) -> Result<Json<Vec<ZoneRating>>, StatusCode> {
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
        eprintln!("Database error getting zone ratings: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
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
    State(state): State<crate::AppState>,
) -> Result<StatusCode, StatusCode> {
    let pool = &*state.zone_state.pool;

    // First, get the rating details before deletion for transaction logging
    let rating_details =
        sqlx::query("SELECT zone_id, user_ip, rating FROM zone_ratings WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(details) = rating_details {
        let zone_id = details.get::<i64, _>("zone_id");
        let user_ip = details.get::<String, _>("user_ip");
        let _old_rating = details.get::<i32, _>("rating") as u8;

        // Delete the rating
        let _ = sqlx::query("DELETE FROM zone_ratings WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Write SQL transaction to log file
        let sql_statement = format!(
            "DELETE FROM zone_ratings WHERE zone_id = {} AND user_ip = '{}'; -- {} - Deleted via admin\n",
            zone_id,
            user_ip.replace("'", "''"), // Escape single quotes
            chrono::Utc::now().to_rfc3339()
        );

        if let Err(e) = write_to_transaction_log(&sql_statement) {
            eprintln!("Warning: Failed to write to transaction log: {}", e);
            // Don't fail the request if logging fails
        }

        // Force WAL checkpoint to immediately update main database file
        let _ = crate::checkpoint_wal(pool).await;

        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::NOT_FOUND)
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
