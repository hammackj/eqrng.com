// Rating-related admin functionality
// This file contains rating management features for the admin interface

#[cfg(feature = "admin")]
use axum::{
    Form,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};

#[cfg(feature = "admin")]
use std::collections::HashMap;

#[cfg(feature = "admin")]
use sqlx::Row;

#[cfg(feature = "admin")]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::admin::types::*;

#[cfg(feature = "admin")]
pub async fn list_all_ratings(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Html<String>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(5, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.clone().unwrap_or_default();
    let sort = params
        .sort
        .clone()
        .unwrap_or_else(|| "created_at".to_string());
    let order = params.order.clone().unwrap_or_else(|| "desc".to_string());

    let pool = &state.zone_state.pool;

    // Validate sort column and order
    let valid_columns = ["id", "zone_id", "rating", "created_at", "updated_at"];
    let sort_column = if valid_columns.contains(&sort.as_str()) {
        sort.as_str()
    } else {
        "created_at"
    };

    let sort_order = if order == "asc" { "ASC" } else { "DESC" };

    // Check if zone_ratings table exists first
    let table_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='zone_ratings'",
    )
    .fetch_one(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if table_exists == 0 {
        return Ok(Html(format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>Manage Ratings - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
    </div>
    <h1>Manage Ratings</h1>
    <p>No ratings table found. The ratings system may not be initialized yet.</p>
    <p>Try creating some zone ratings first from the main application.</p>
</body>
</html>
"#
        )));
    }

    // Get total count - simplified query with error fallback
    let total_count: i64 = if search.is_empty() {
        sqlx::query_scalar("SELECT COUNT(*) FROM zone_ratings")
            .fetch_one(pool.as_ref())
            .await
            .unwrap_or(0)
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM zone_ratings LEFT JOIN zones ON zone_ratings.zone_id = zones.id WHERE zones.name LIKE ?"
        )
        .bind(format!("%{}%", search))
        .fetch_one(pool.as_ref())
        .await
        .unwrap_or(0)
    };

    // Get ratings with zone names - simplified query with better error handling
    let rating_rows = if search.is_empty() {
        sqlx::query(&format!(
            r#"
            SELECT
                zone_ratings.id,
                zone_ratings.zone_id,
                zone_ratings.rating,
                zone_ratings.created_at,
                zone_ratings.updated_at,
                COALESCE(zones.name, 'Unknown Zone') as zone_name
            FROM zone_ratings
            LEFT JOIN zones ON zone_ratings.zone_id = zones.id
            ORDER BY zone_ratings.{} {}
            LIMIT ? OFFSET ?
            "#,
            sort_column, sort_order
        ))
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default()
    } else {
        sqlx::query(&format!(
            r#"
            SELECT
                zone_ratings.id,
                zone_ratings.zone_id,
                zone_ratings.rating,
                zone_ratings.created_at,
                zone_ratings.updated_at,
                COALESCE(zones.name, 'Unknown Zone') as zone_name
            FROM zone_ratings
            LEFT JOIN zones ON zone_ratings.zone_id = zones.id
            WHERE zones.name LIKE ?
            ORDER BY zone_ratings.{} {}
            LIMIT ? OFFSET ?
            "#,
            sort_column, sort_order
        ))
        .bind(format!("%{}%", search))
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default()
    };

    let total_pages = (total_count as i32 + per_page - 1) / per_page;

    // Generate sort links
    let make_sort_link = |column: &str, display_name: &str| {
        let new_order = if sort == column && order == "asc" {
            "desc"
        } else {
            "asc"
        };
        let arrow = if sort == column {
            if order == "asc" { " ↑" } else { " ↓" }
        } else {
            ""
        };

        let mut query_params = vec![
            format!("sort={}", column),
            format!("order={}", new_order),
            format!("per_page={}", per_page),
        ];
        if !search.is_empty() {
            query_params.push(format!("search={}", urlencoding::encode(&search)));
        }

        format!(
            r#"<a href="/admin/ratings?{}" style="text-decoration: none; color: inherit;">{}{}</a>"#,
            query_params.join("&"),
            display_name,
            arrow
        )
    };

    let mut html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Manage Ratings - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .controls {{ margin-bottom: 20px; display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }}
        .controls input {{ padding: 8px; border: 1px solid #ddd; border-radius: 4px; }}
        .btn {{ background: #007bff; color: white; padding: 8px 15px; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }}
        .btn:hover {{ background: #0056b3; }}
        .btn-danger {{ background: #dc3545; }}
        .btn-danger:hover {{ background: #c82333; }}
        .btn-small {{ padding: 4px 8px; font-size: 0.8em; }}
        table {{ width: 100%; border-collapse: collapse; margin-bottom: 20px; }}
        th, td {{ padding: 8px; border: 1px solid #ddd; text-align: left; }}
        th {{ background: #f8f9fa; border-bottom: 2px solid #dee2e6; }}
        th a:hover {{ background-color: #e9ecef; padding: 4px; border-radius: 3px; }}
        .pagination {{ display: flex; gap: 5px; align-items: center; }}
        .pagination a {{ padding: 8px 12px; text-decoration: none; border: 1px solid #ddd; border-radius: 4px; }}
        .pagination a.current {{ background: #007bff; color: white; }}
        .rating-stars {{ color: #ffc107; }}
        .text-center {{ text-align: center; }}
    </style>
    <script>
        function deleteRating(id, zoneName) {{
            if (confirm('Are you sure you want to delete this rating for "' + zoneName + '"?')) {{
                document.getElementById('delete-form-' + id).submit();
            }}
        }}
    </script>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/instances">Manage Instances</a>
        <a href="/admin/ratings">Manage Ratings</a>
        <a href="/admin/links">Manage Links</a>
        <a href="/admin/note-types">Note Types</a>
        <a href="/admin/flag-types">Flag Types</a>
    </div>

    <h1>Manage Ratings</h1>

    <div class="controls">
        <form method="get" style="display: flex; gap: 10px; align-items: center;">
            <input type="text" name="search" placeholder="Search by zone name..." value="{}" />
            <input type="hidden" name="page" value="1" />
            <input type="hidden" name="per_page" value="{}" />
            <button type="submit" class="btn">Search</button>
        </form>
    </div>

    <p>Showing {} ratings (Page {} of {})</p>

    <table>
        <thead>
            <tr>
                <th class="text-center">{}</th>
                <th>{}</th>
                <th class="text-center">{}</th>
                <th class="text-center">{}</th>
                <th class="text-center">{}</th>
                <th class="text-center">Actions</th>
            </tr>
        </thead>
        <tbody>
"#,
        search,
        per_page,
        total_count as i32,
        page,
        total_pages,
        make_sort_link("id", "ID"),
        make_sort_link("zone_id", "Zone Name"),
        make_sort_link("rating", "Rating"),
        make_sort_link("created_at", "Created"),
        make_sort_link("updated_at", "Updated"),
    );

    // Add rating rows
    for row in rating_rows {
        let id: i64 = row.get("id");
        let zone_id: i64 = row.get("zone_id");
        let rating: i32 = row.get("rating");
        let zone_name: String = row
            .get::<Option<String>, _>("zone_name")
            .unwrap_or_else(|| format!("Zone #{}", zone_id));
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");

        // Format rating as stars
        let rating_stars = "★".repeat(rating as usize) + &"☆".repeat((5 - rating) as usize);

        html.push_str(&format!(
            r#"
            <tr>
                <td class="text-center">{}</td>
                <td><a href="/admin/zones/{}" style="text-decoration: none; color: #007bff;">{}</a></td>
                <td class="text-center"><span class="rating-stars" title="{}/5">{}</span></td>
                <td class="text-center">{}</td>
                <td class="text-center">{}</td>
                <td class="text-center">
                    <button onclick="deleteRating({}, '{}')" class="btn btn-danger btn-small">Delete</button>
                    <form id="delete-form-{}" method="post" action="/admin/ratings/{}/delete" style="display: none;">
                        <input type="hidden" name="_method" value="DELETE" />
                    </form>
                </td>
            </tr>
            "#,
            id,
            zone_id,
            zone_name,
            rating,
            rating_stars,
            created_at.split('T').next().unwrap_or(&created_at),
            updated_at.split('T').next().unwrap_or(&updated_at),
            id,
            zone_name.replace("'", "\\'"),
            id,
            id
        ));
    }

    html.push_str("</tbody></table>");

    // Add pagination
    if total_pages > 1 {
        html.push_str(r#"<div class="pagination">"#);

        if page > 1 {
            let mut prev_params = vec![
                format!("page={}", page - 1),
                format!("per_page={}", per_page),
            ];
            if !search.is_empty() {
                prev_params.push(format!("search={}", urlencoding::encode(&search)));
            }
            if sort != "created_at" {
                prev_params.push(format!("sort={}", sort));
            }
            if order != "desc" {
                prev_params.push(format!("order={}", order));
            }

            html.push_str(&format!(
                r#"<a href="/admin/ratings?{}">« Previous</a>"#,
                prev_params.join("&")
            ));
        }

        for p in 1..=total_pages {
            if p == page {
                html.push_str(&format!(r#"<span class="pagination current">{}</span>"#, p));
            } else {
                let mut page_params = vec![format!("page={}", p), format!("per_page={}", per_page)];
                if !search.is_empty() {
                    page_params.push(format!("search={}", urlencoding::encode(&search)));
                }
                if sort != "created_at" {
                    page_params.push(format!("sort={}", sort));
                }
                if order != "desc" {
                    page_params.push(format!("order={}", order));
                }

                html.push_str(&format!(
                    r#"<a href="/admin/ratings?{}">{}</a>"#,
                    page_params.join("&"),
                    p
                ));
            }
        }

        if page < total_pages {
            let mut next_params = vec![
                format!("page={}", page + 1),
                format!("per_page={}", per_page),
            ];
            if !search.is_empty() {
                next_params.push(format!("search={}", urlencoding::encode(&search)));
            }
            if sort != "created_at" {
                next_params.push(format!("sort={}", sort));
            }
            if order != "desc" {
                next_params.push(format!("order={}", order));
            }

            html.push_str(&format!(
                r#"<a href="/admin/ratings?{}">Next »</a>"#,
                next_params.join("&")
            ));
        }

        html.push_str("</div>");
    }

    html.push_str("</body></html>");

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn handle_rating_delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(_form): Form<HashMap<String, String>>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Delete the rating
    sqlx::query("DELETE FROM zone_ratings WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/admin/ratings"))
}

#[cfg(feature = "admin")]
pub async fn delete_rating_admin(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Delete the rating
    sqlx::query("DELETE FROM zone_ratings WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/admin/ratings"))
}
