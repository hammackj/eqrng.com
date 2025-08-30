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
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, warn};

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

#[cfg(test)]
pub(crate) fn test_hash_ip(ip: &str, config: &crate::AppConfig) -> String {
    hash_ip(ip, config)
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
    let user_rating =
        sqlx::query("SELECT rating FROM zone_ratings WHERE zone_id = ? AND user_ip = ?")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classes::ClassRaceState;
    use crate::config::{
        AdminConfig, AppConfig, CorsConfig, DatabaseConfig, LoggingConfig, RatingsConfig,
        SecurityConfig, ServerConfig,
    };
    use crate::instances::InstanceState;
    use crate::zones::ZoneState;
    use crate::{AppError, AppState};
    use axum::Json;
    use axum::extract::{ConnectInfo, Path, State};
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::{Row, SqlitePool};
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::Arc;

    fn test_config_with_key(key: &str) -> AppConfig {
        AppConfig {
            server: ServerConfig {
                port: 0,
                host: "localhost".to_string(),
            },
            database: DatabaseConfig {
                path: "".to_string(),
                backup_dir: "".to_string(),
                migrate_on_startup: false,
            },
            security: SecurityConfig {
                rating_ip_hash_key: key.to_string(),
                min_ip_hash_key_length: 0,
            },
            ratings: RatingsConfig {
                min_rating: 1,
                max_rating: 5,
                transaction_log_path: "".to_string(),
            },
            admin: AdminConfig {
                enabled: false,
                page_size: 10,
                min_page_size: 1,
                max_page_size: 100,
                default_sort_column: "id".to_string(),
                default_sort_order: "asc".to_string(),
            },
            cors: CorsConfig {
                development_origins: vec![],
                production_origins: vec![],
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "text".to_string(),
                file_path: "".to_string(),
                max_file_size: "1MB".to_string(),
                max_files: 1,
            },
        }
    }

    async fn setup_state() -> (AppState, Arc<SqlitePool>) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query("CREATE TABLE zones (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            r#"
            CREATE TABLE zone_ratings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                zone_id INTEGER NOT NULL,
                user_ip TEXT NOT NULL,
                rating INTEGER NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(zone_id, user_ip)
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO zones (id, name) VALUES (1, 'Test Zone')")
            .execute(&pool)
            .await
            .unwrap();

        let pool_arc = Arc::new(pool);
        let state = AppState {
            config: Arc::new(test_config_with_key("testkey")),
            zone_state: ZoneState {
                pool: pool_arc.clone(),
            },
            instance_state: InstanceState {
                pool: pool_arc.clone(),
            },
            class_race_state: ClassRaceState {
                class_race_map: Arc::new(HashMap::new()),
            },
        };
        (state, pool_arc)
    }

    #[test]
    fn hash_ip_is_stable_and_key_dependent() {
        let ip = "127.0.0.1";
        let config1 = test_config_with_key("samekey");
        let h1 = super::test_hash_ip(ip, &config1);
        let h2 = super::test_hash_ip(ip, &config1);
        assert_eq!(h1, h2);

        let config2 = test_config_with_key("differentkey");
        let h3 = super::test_hash_ip(ip, &config2);
        assert_ne!(h1, h3);
    }

    #[tokio::test]
    async fn submit_zone_rating_rejects_out_of_range() {
        let (state, _) = setup_state().await;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let result = submit_zone_rating(
            Path(1),
            ConnectInfo(addr),
            State(state),
            Json(SubmitRatingRequest { rating: 0 }),
        )
        .await;
        assert!(matches!(result, Err(AppError::InvalidRating(0, 1, 5))));
    }

    #[tokio::test]
    async fn submit_zone_rating_accepts_valid_rating() {
        let (state, pool) = setup_state().await;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let rating = 5;
        let result = submit_zone_rating(
            Path(1),
            ConnectInfo(addr),
            State(state),
            Json(SubmitRatingRequest { rating }),
        )
        .await;
        assert!(result.is_ok());

        let row = sqlx::query("SELECT COUNT(*) as count FROM zone_ratings")
            .fetch_one(&*pool)
            .await
            .unwrap();
        let count: i64 = row.get("count");
        assert_eq!(count, 1);

        let row = sqlx::query("SELECT rating FROM zone_ratings WHERE zone_id = 1")
            .fetch_one(&*pool)
            .await
            .unwrap();
        let stored: i64 = row.get("rating");
        assert_eq!(stored as u8, rating);
    }
}
