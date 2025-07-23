use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

use crate::AppState;

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

    let result = sqlx::query(
        "INSERT INTO links (name, url, category, description)
         VALUES (?, ?, ?, ?)
         RETURNING id, name, url, category, description, created_at",
    )
    .bind(&form.name)
    .bind(&form.url)
    .bind(&form.category)
    .bind(&form.description)
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

    let result = sqlx::query(
        "UPDATE links
         SET name = ?, url = ?, category = ?, description = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?
         RETURNING id, name, url, category, description, created_at",
    )
    .bind(&form.name)
    .bind(&form.url)
    .bind(&form.category)
    .bind(&form.description)
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
