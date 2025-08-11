// Links-related admin functionality
// This file contains link management features for the admin interface

#[cfg(feature = "admin")]
use crate::security::{escape_html, sanitize_url, sanitize_user_input};

#[cfg(feature = "admin")]
use axum::{
    Form,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};

#[cfg(feature = "admin")]
use sqlx::Row;

#[cfg(feature = "admin")]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::admin::types::*;

#[cfg(feature = "admin")]
pub async fn list_links(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Html<String>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(5, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.clone().unwrap_or_default();
    let sort = params.sort.clone().unwrap_or_else(|| "name".to_string());
    let order = params.order.clone().unwrap_or_else(|| "asc".to_string());

    let pool = &state.zone_state.pool;

    // Validate sort column and order
    let valid_columns = ["id", "name", "category", "created_at", "updated_at"];
    let sort_column = if valid_columns.contains(&sort.as_str()) {
        sort.as_str()
    } else {
        "name"
    };

    let sort_order = if order == "asc" { "ASC" } else { "DESC" };

    // Build search query
    let mut where_clause = String::new();
    let search_param = if !search.is_empty() {
        where_clause = "WHERE (name LIKE ? OR category LIKE ? OR description LIKE ?)".to_string();
        Some(format!("%{}%", search))
    } else {
        None
    };

    // Get total count
    let count_query = format!(
        r#"
        SELECT COUNT(*) as count
        FROM links
        {}
        "#,
        where_clause
    );

    let mut count_query_builder = sqlx::query(&count_query);
    if let Some(ref search_val) = search_param {
        count_query_builder = count_query_builder
            .bind(search_val)
            .bind(search_val)
            .bind(search_val);
    }

    let total_count: i32 = count_query_builder
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get("count");

    // Get links
    let links_query = format!(
        r#"
        SELECT id, name, url, category, description, created_at, updated_at
        FROM links
        {}
        ORDER BY {} {}
        LIMIT ? OFFSET ?
        "#,
        where_clause, sort_column, sort_order
    );

    let mut links_query_builder = sqlx::query(&links_query);
    if let Some(ref search_val) = search_param {
        links_query_builder = links_query_builder
            .bind(search_val)
            .bind(search_val)
            .bind(search_val);
    }

    let link_rows = links_query_builder
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total_pages = (total_count + per_page - 1) / per_page;

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
            r#"<a href="/admin/links?{}" style="text-decoration: none; color: inherit;">{}{}</a>"#,
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
    <title>Manage Links - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1400px; margin: 0 auto; padding: 20px; }}
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
        .truncate {{ max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
        .category-badge {{ display: inline-block; padding: 4px 8px; border-radius: 12px; background: #007bff; color: white; font-size: 0.8em; }}
    </style>
    <script>
        function deleteLink(id, name) {{
            if (confirm('Are you sure you want to delete "' + name + '"?')) {{
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

    <h1>Manage Links</h1>

    <div class="controls">
        <form method="get" style="display: flex; gap: 10px; align-items: center;">
            <input type="text" name="search" placeholder="Search links..." value="{}" />
            <input type="hidden" name="page" value="1" />
            <input type="hidden" name="per_page" value="{}" />
            <button type="submit" class="btn">Search</button>
        </form>
        <a href="/admin/links/new" class="btn">Add New Link</a>
    </div>

    <p>Showing {} links (Page {} of {})</p>

    <table>
        <thead>
            <tr>
                <th>{}</th>
                <th>{}</th>
                <th>URL</th>
                <th>{}</th>
                <th>Description</th>
                <th>{}</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
"#,
        search,
        per_page,
        total_count,
        page,
        total_pages,
        make_sort_link("name", "Name"),
        make_sort_link("category", "Category"),
        make_sort_link("category", "Category"),
        make_sort_link("created_at", "Created"),
    );

    // Add link rows
    for row in link_rows {
        let id: i64 = row.get("id");
        let name: String = row.get("name");
        let url: String = row.get("url");
        let category: String = row.get("category");
        let description: Option<String> = row.get("description");
        let created_at: String = row.get("created_at");

        html.push_str(&format!(
            r#"
            <tr>
                <td>{}</td>
                <td><span class="category-badge">{}</span></td>
                <td><a href="{}" target="_blank" style="color: #007bff;">{}</a></td>
                <td>{}</td>
                <td class="truncate">{}</td>
                <td>{}</td>
                <td>
                    <a href="/admin/links/{}" class="btn btn-small">Edit</a>
                    <button onclick="deleteLink({}, '{}')" class="btn btn-danger btn-small">Delete</button>
                    <form id="delete-form-{}" method="post" action="/admin/links/{}/delete" style="display: none;">
                        <input type="hidden" name="_method" value="DELETE" />
                    </form>
                </td>
            </tr>
            "#,
            name,
            category,
            url,
            if url.len() > 50 {
                format!("{}...", &url[..50])
            } else {
                url.clone()
            },
            category,
            description.unwrap_or_else(|| "N/A".to_string()),
            created_at.split('T').next().unwrap_or(&created_at),
            id,
            id,
            name.replace("'", "\\'"),
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
            if sort != "name" {
                prev_params.push(format!("sort={}", sort));
            }
            if order != "asc" {
                prev_params.push(format!("order={}", order));
            }

            html.push_str(&format!(
                r#"<a href="/admin/links?{}">« Previous</a>"#,
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
                if sort != "name" {
                    page_params.push(format!("sort={}", sort));
                }
                if order != "asc" {
                    page_params.push(format!("order={}", order));
                }

                html.push_str(&format!(
                    r#"<a href="/admin/links?{}">{}</a>"#,
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
            if sort != "name" {
                next_params.push(format!("sort={}", sort));
            }
            if order != "asc" {
                next_params.push(format!("order={}", order));
            }

            html.push_str(&format!(
                r#"<a href="/admin/links?{}">Next »</a>"#,
                next_params.join("&")
            ));
        }

        html.push_str("</div>");
    }

    html.push_str("</body></html>");

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn new_link_form() -> Result<Html<String>, StatusCode> {
    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Add New Link - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .form-group {{ margin-bottom: 20px; }}
        label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        input, select, textarea {{ width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }}
        textarea {{ height: 100px; resize: vertical; }}
        .btn {{ background: #007bff; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }}
        .btn:hover {{ background: #0056b3; }}
        .btn-secondary {{ background: #6c757d; }}
        .btn-secondary:hover {{ background: #545b62; }}
    </style>
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

    <h1>Add New Link</h1>

    <form method="post" action="/admin/links">
        <div class="form-group">
            <label for="name">Name:</label>
            <input type="text" id="name" name="name" required />
        </div>

        <div class="form-group">
            <label for="url">URL:</label>
            <input type="url" id="url" name="url" required />
        </div>

        <div class="form-group">
            <label for="category">Category:</label>
            <select id="category" name="category" required>
                <option value="Tools">Tools</option>
                <option value="Guides">Guides</option>
                <option value="Maps">Maps</option>
                <option value="Databases">Databases</option>
                <option value="Forums">Forums</option>
                <option value="Resources">Resources</option>
                <option value="Other">Other</option>
            </select>
        </div>

        <div class="form-group">
            <label for="description">Description:</label>
            <textarea id="description" name="description" placeholder="Optional description..."></textarea>
        </div>

        <div class="form-group">
            <button type="submit" class="btn">Create Link</button>
            <a href="/admin/links" class="btn btn-secondary">Cancel</a>
        </div>
    </form>
</body>
</html>
"#
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn edit_link_form(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get the link
    let link_row = sqlx::query("SELECT * FROM links WHERE id = ?")
        .bind(id)
        .fetch_optional(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let name: String = link_row.get("name");
    let url: String = link_row.get("url");
    let category: String = link_row.get("category");
    let description: Option<String> = link_row.get("description");

    let categories = [
        "Tools",
        "Guides",
        "Maps",
        "Databases",
        "Forums",
        "Resources",
        "Other",
    ];
    let category_options = categories
        .iter()
        .map(|cat| {
            if *cat == category {
                format!(r#"<option value="{}" selected>{}</option>"#, cat, cat)
            } else {
                format!(r#"<option value="{}">{}</option>"#, cat, cat)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Edit Link - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .form-group {{ margin-bottom: 20px; }}
        label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        input, select, textarea {{ width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }}
        textarea {{ height: 100px; resize: vertical; }}
        .btn {{ background: #007bff; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }}
        .btn:hover {{ background: #0056b3; }}
        .btn-secondary {{ background: #6c757d; }}
        .btn-secondary:hover {{ background: #545b62; }}
        .btn-danger {{ background: #dc3545; }}
        .btn-danger:hover {{ background: #c82333; }}
    </style>
    <script>
        function deleteLink() {{
            if (confirm('Are you sure you want to delete this link?')) {{
                document.getElementById('delete-form').submit();
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

    <h1>Edit Link</h1>

    <form method="post" action="/admin/links/{}">
        <input type="hidden" name="_method" value="PUT" />

        <div class="form-group">
            <label for="name">Name:</label>
            <input type="text" id="name" name="name" value="{}" required />
        </div>

        <div class="form-group">
            <label for="url">URL:</label>
            <input type="url" id="url" name="url" value="{}" required />
        </div>

        <div class="form-group">
            <label for="category">Category:</label>
            <select id="category" name="category" required>
                {}
            </select>
        </div>

        <div class="form-group">
            <label for="description">Description:</label>
            <textarea id="description" name="description" placeholder="Optional description...">{}</textarea>
        </div>

        <div class="form-group">
            <button type="submit" class="btn">Update Link</button>
            <a href="/admin/links" class="btn btn-secondary">Cancel</a>
            <button type="button" onclick="deleteLink()" class="btn btn-danger">Delete</button>
        </div>
    </form>

    <form id="delete-form" method="post" action="/admin/links/{}/delete" style="display: none;">
        <input type="hidden" name="_method" value="DELETE" />
    </form>
</body>
</html>
"#,
        id,
        escape_html(&name),
        escape_html(&url),
        category_options,
        escape_html(&description.unwrap_or_default()),
        id
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn create_link_admin(
    State(state): State<AppState>,
    Form(form): Form<LinkForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Insert the new link
    sqlx::query(
        r#"
        INSERT INTO links (name, url, category, description)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&form.name)
    .bind(&form.url)
    .bind(&form.category)
    .bind(&form.description)
    .execute(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/admin/links"))
}

#[cfg(feature = "admin")]
pub async fn handle_link_update_or_delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<LinkForm>,
) -> Result<Redirect, StatusCode> {
    if let Some(ref method) = form._method {
        if method == "DELETE" {
            return delete_link_admin(State(state), Path(id)).await;
        }
    }

    update_link(State(state), Path(id), Form(form)).await
}

#[cfg(feature = "admin")]
pub async fn update_link(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<LinkForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Update the link
    sqlx::query(
        r#"
        UPDATE links
        SET name = ?, url = ?, category = ?, description = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(&form.name)
    .bind(&form.url)
    .bind(&form.category)
    .bind(&form.description)
    .bind(id)
    .execute(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/admin/links"))
}

#[cfg(feature = "admin")]
pub async fn delete_link_admin(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Delete the link
    sqlx::query("DELETE FROM links WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/admin/links"))
}
