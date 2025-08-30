use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

use crate::{AppState, security};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Link {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub category: String,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LinkForm {
    pub name: String,
    pub url: String,
    pub category: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LinksQuery {
    pub category: Option<String>,
}

pub async fn get_links(
    State(state): State<AppState>,
    Query(query): Query<LinksQuery>,
) -> Result<Json<Vec<Link>>, StatusCode> {
    let pool = &state.zone_state.pool;

    let links = if let Some(category) = query.category {
        sqlx::query(
            "SELECT id, name, url, category, description, created_at
             FROM links
             WHERE category = ?
             ORDER BY category, name COLLATE NOCASE",
        )
        .bind(&category)
        .fetch_all(pool.as_ref())
        .await
    } else {
        sqlx::query(
            "SELECT id, name, url, category, description, created_at
             FROM links
             ORDER BY category, name COLLATE NOCASE",
        )
        .fetch_all(pool.as_ref())
        .await
    };

    match links {
        Ok(rows) => {
            let links: Vec<Link> = rows
                .into_iter()
                .map(|row| Link {
                    id: row.get("id"),
                    name: row.get("name"),
                    url: row.get("url"),
                    category: row.get("category"),
                    description: row.get("description"),
                    created_at: row.get("created_at"),
                })
                .collect();

            Ok(Json(links))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_links_by_category(
    State(state): State<AppState>,
) -> Result<Json<HashMap<String, Vec<Link>>>, StatusCode> {
    let pool = &state.zone_state.pool;

    let links = sqlx::query(
        "SELECT id, name, url, category, description, created_at
         FROM links
         ORDER BY category, name COLLATE NOCASE",
    )
    .fetch_all(pool.as_ref())
    .await;

    match links {
        Ok(rows) => {
            let mut categorized_links: HashMap<String, Vec<Link>> = HashMap::new();

            for row in rows {
                let link = Link {
                    id: row.get("id"),
                    name: row.get("name"),
                    url: row.get("url"),
                    category: row.get("category"),
                    description: row.get("description"),
                    created_at: row.get("created_at"),
                };

                categorized_links
                    .entry(link.category.clone())
                    .or_insert_with(Vec::new)
                    .push(link);
            }

            Ok(Json(categorized_links))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Link>, StatusCode> {
    let pool = &state.zone_state.pool;

    let link = sqlx::query(
        "SELECT id, name, url, category, description, created_at
         FROM links
         WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool.as_ref())
    .await;

    match link {
        Ok(Some(row)) => {
            let link = Link {
                id: row.get("id"),
                name: row.get("name"),
                url: row.get("url"),
                category: row.get("category"),
                description: row.get("description"),
                created_at: row.get("created_at"),
            };
            Ok(Json(link))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn create_link(
    State(state): State<AppState>,
    Json(form): Json<LinkForm>,
) -> Result<Json<Link>, StatusCode> {
    let pool = &state.zone_state.pool;

    let valid_categories = ["General", "Class Discords", "Content Creators"];
    if !valid_categories.contains(&form.category.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let url = match security::sanitize_url(&form.url) {
        Some(url) => url,
        None => return Err(StatusCode::BAD_REQUEST),
    };
    let name = security::sanitize_user_input(&form.name);
    let description = form
        .description
        .as_ref()
        .map(|d| security::sanitize_user_input_with_formatting(d));

    let result = sqlx::query(
        "INSERT INTO links (name, url, category, description)
         VALUES (?, ?, ?, ?)
         RETURNING id, name, url, category, description, created_at",
    )
    .bind(&name)
    .bind(&url)
    .bind(&form.category)
    .bind(&description)
    .fetch_one(pool.as_ref())
    .await;

    match result {
        Ok(row) => {
            let link = Link {
                id: row.get("id"),
                name: row.get("name"),
                url: row.get("url"),
                category: row.get("category"),
                description: row.get("description"),
                created_at: row.get("created_at"),
            };
            Ok(Json(link))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(form): Json<LinkForm>,
) -> Result<Json<Link>, StatusCode> {
    let pool = &state.zone_state.pool;

    let valid_categories = ["General", "Class Discords", "Content Creators"];
    if !valid_categories.contains(&form.category.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let url = match security::sanitize_url(&form.url) {
        Some(url) => url,
        None => return Err(StatusCode::BAD_REQUEST),
    };
    let name = security::sanitize_user_input(&form.name);
    let description = form
        .description
        .as_ref()
        .map(|d| security::sanitize_user_input_with_formatting(d));

    let result = sqlx::query(
        "UPDATE links
         SET name = ?, url = ?, category = ?, description = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?
         RETURNING id, name, url, category, description, created_at",
    )
    .bind(&name)
    .bind(&url)
    .bind(&form.category)
    .bind(&description)
    .bind(id)
    .fetch_optional(pool.as_ref())
    .await;

    match result {
        Ok(Some(row)) => {
            let link = Link {
                id: row.get("id"),
                name: row.get("name"),
                url: row.get("url"),
                category: row.get("category"),
                description: row.get("description"),
                created_at: row.get("created_at"),
            };
            Ok(Json(link))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn delete_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let pool = &state.zone_state.pool;

    let result = sqlx::query("DELETE FROM links WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_categories() -> Result<Json<Vec<String>>, StatusCode> {
    let categories = vec![
        "General".to_string(),
        "Class Discords".to_string(),
        "Content Creators".to_string(),
    ];
    Ok(Json(categories))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::config::{
        AdminConfig, AppConfig, CorsConfig, DatabaseConfig, LoggingConfig, RatingsConfig,
        SecurityConfig, ServerConfig,
    };
    use crate::{classes, instances, zones};

    async fn setup_state() -> AppState {
        let pool = sqlx::SqlitePool::connect(":memory:")
            .await
            .expect("failed to create pool");

        sqlx::query(
            r#"CREATE TABLE links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(&pool)
        .await
        .expect("failed to create table");

        let pool = Arc::new(pool);

        let config = Arc::new(AppConfig {
            server: ServerConfig {
                port: 0,
                host: "localhost".into(),
            },
            database: DatabaseConfig {
                path: String::new(),
                backup_dir: String::new(),
                migrate_on_startup: false,
            },
            security: SecurityConfig {
                rating_ip_hash_key: "key".into(),
                min_ip_hash_key_length: 0,
            },
            ratings: RatingsConfig {
                min_rating: 0,
                max_rating: 5,
                transaction_log_path: String::new(),
            },
            admin: AdminConfig {
                enabled: false,
                page_size: 0,
                min_page_size: 0,
                max_page_size: 0,
                default_sort_column: String::new(),
                default_sort_order: String::new(),
            },
            cors: CorsConfig {
                development_origins: vec![],
                production_origins: vec![],
            },
            logging: LoggingConfig {
                level: String::new(),
                format: String::new(),
                file_path: String::new(),
                max_file_size: String::new(),
                max_files: 0,
            },
        });

        AppState {
            config,
            zone_state: zones::ZoneState { pool: pool.clone() },
            instance_state: instances::InstanceState { pool: pool.clone() },
            class_race_state: classes::ClassRaceState {
                class_race_map: Arc::new(HashMap::new()),
            },
        }
    }

    #[tokio::test]
    async fn create_link_persists_and_sanitizes() {
        let state = setup_state().await;
        let form = LinkForm {
            name: "<script>bad</script>Good".into(),
            url: "https://example.com".into(),
            category: "General".into(),
            description: Some("<b>Bold</b><script>x</script>".into()),
        };

        let Json(link) = create_link(State(state.clone()), Json(form))
            .await
            .expect("create link failed");

        let row = sqlx::query("SELECT name, description FROM links WHERE id = ?")
            .bind(link.id)
            .fetch_one(state.zone_state.pool.as_ref())
            .await
            .expect("query failed");

        let name: String = row.get("name");
        let description: Option<String> = row.get("description");

        assert_eq!(name, "Good");
        assert_eq!(description.as_deref(), Some("<b>Bold</b>x"));
    }

    #[tokio::test]
    async fn create_link_rejects_malicious_url() {
        let state = setup_state().await;
        let form = LinkForm {
            name: "Test".into(),
            url: "javascript:alert(1)".into(),
            category: "General".into(),
            description: None,
        };

        let result = create_link(State(state), Json(form)).await;
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn update_link_persists_and_sanitizes() {
        let state = setup_state().await;
        let form = LinkForm {
            name: "Initial".into(),
            url: "https://example.com".into(),
            category: "General".into(),
            description: None,
        };
        let Json(link) = create_link(State(state.clone()), Json(form))
            .await
            .expect("create link failed");

        let update_form = LinkForm {
            name: "<b>Updated</b><script>bad</script>".into(),
            url: "https://example.org".into(),
            category: "General".into(),
            description: Some("<i>Desc</i><script>x</script>".into()),
        };
        let Json(updated) = update_link(State(state.clone()), Path(link.id), Json(update_form))
            .await
            .expect("update link failed");

        let row = sqlx::query("SELECT name, url, description FROM links WHERE id = ?")
            .bind(updated.id)
            .fetch_one(state.zone_state.pool.as_ref())
            .await
            .expect("query failed");

        let name: String = row.get("name");
        let url: String = row.get("url");
        let description: Option<String> = row.get("description");

        assert_eq!(name, "Updated");
        assert_eq!(url, "https://example.org");
        assert_eq!(description.as_deref(), Some("<i>Desc</i>x"));
    }

    #[tokio::test]
    async fn update_link_rejects_malicious_url() {
        let state = setup_state().await;
        let form = LinkForm {
            name: "Initial".into(),
            url: "https://example.com".into(),
            category: "General".into(),
            description: None,
        };
        let Json(link) = create_link(State(state.clone()), Json(form))
            .await
            .expect("create link failed");

        let bad_form = LinkForm {
            name: "Bad".into(),
            url: "javascript:alert(1)".into(),
            category: "General".into(),
            description: None,
        };

        let result = update_link(State(state), Path(link.id), Json(bad_form)).await;
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }
}
