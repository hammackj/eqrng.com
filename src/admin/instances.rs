// Instance management functionality
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
use urlencoding;

#[cfg(feature = "admin")]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::admin::dashboard::generate_sortable_header;
#[cfg(feature = "admin")]
use crate::admin::types::*;
#[cfg(feature = "admin")]
use crate::security;

#[cfg(feature = "admin")]
pub async fn list_instances(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Html<String>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(5, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.unwrap_or_default();
    let sort = params.sort.clone().unwrap_or_else(|| "name".to_string());
    let order = params.order.clone().unwrap_or_else(|| "asc".to_string());
    let verified = params.verified.clone();

    let pool = &state.instance_state.pool;

    // Validate sort column and order
    let valid_columns = [
        "id",
        "name",
        "level_ranges",
        "expansion",
        "zone_type",
        "rating",
        "hot_zone",
        "verified",
        "created_at",
    ];
    let sort_column = if valid_columns.contains(&sort.as_str()) {
        sort.as_str()
    } else {
        "name"
    };

    let sort_order = if order == "asc" { "ASC" } else { "DESC" };

    // Build search query
    let mut where_conditions = Vec::new();
    let mut search_param = None;

    if !search.is_empty() {
        where_conditions.push("(name LIKE ? OR expansion LIKE ? OR zone_type LIKE ?)".to_string());
        search_param = Some(format!("%{}%", search));
    }

    if let Some(ref verified_param) = verified {
        if verified_param == "true" {
            where_conditions.push("verified = 1".to_string());
        } else if verified_param == "false" {
            where_conditions.push("verified = 0".to_string());
        }
    }

    let where_clause = if where_conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", where_conditions.join(" AND "))
    };

    // Get total count
    let count_query = format!("SELECT COUNT(*) as count FROM instances {}", where_clause);
    let total_count: i32 = if let Some(ref search_term) = search_param {
        sqlx::query(&count_query)
            .bind(search_term)
            .bind(search_term)
            .bind(search_term)
            .fetch_one(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get("count")
    } else {
        sqlx::query(&count_query)
            .fetch_one(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get("count")
    };

    // Get instances
    let instances_query = format!(
        "SELECT id, name, level_ranges, expansion, continent, zone_type, connections, image_url, map_url, rating, hot_zone, verified FROM instances {} ORDER BY {} {} LIMIT ? OFFSET ?",
        where_clause, sort_column, sort_order
    );

    let instance_rows = if let Some(ref search_term) = search_param {
        sqlx::query(&instances_query)
            .bind(search_term)
            .bind(search_term)
            .bind(search_term)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        sqlx::query(&instances_query)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    // Load notes for each instance
    let mut instances = Vec::new();
    for row in instance_rows {
        let instance_id: i64 = row.get("id");
        let notes = crate::instances::get_instance_notes(pool.as_ref(), instance_id)
            .await
            .unwrap_or_default();

        instances.push(Instance {
            id: Some(instance_id as i32),
            name: row.get("name"),
            level_ranges: row.get("level_ranges"),
            expansion: row.get("expansion"),
            continent: row.get("continent"),
            zone_type: row.get("zone_type"),
            connections: row.get("connections"),
            image_url: row.get("image_url"),
            map_url: row.get("map_url"),
            rating: row.get("rating"),
            hot_zone: row.get("hot_zone"),
            verified: row.get("verified"),
            notes,
        });
    }

    let total_pages = (total_count + per_page - 1) / per_page;

    let mut html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Manage Instances - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1400px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .controls {{ margin-bottom: 20px; display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }}
        .controls input, .controls select {{ padding: 8px; border: 1px solid #ddd; border-radius: 4px; }}
        .btn {{ background: #007bff; color: white; padding: 8px 15px; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }}
        .btn:hover {{ background: #0056b3; }}
        .btn-danger {{ background: #dc3545; }}
        .btn-danger:hover {{ background: #c82333; }}
        .btn-small {{ padding: 4px 8px; font-size: 0.8em; }}
        table {{ width: 100%; border-collapse: collapse; margin-bottom: 20px; }}
        th, td {{ padding: 8px; border: 1px solid #ddd; text-align: left; }}
        th {{ background: #f5f5f5; }}
        .pagination {{ display: flex; gap: 5px; align-items: center; }}
        .pagination a {{ padding: 8px 12px; text-decoration: none; border: 1px solid #ddd; border-radius: 4px; }}
        .pagination a.current {{ background: #007bff; color: white; }}
        .truncate {{ max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
        th {{ background-color: #f8f9fa; border-bottom: 2px solid #dee2e6; }}
        th a:hover {{ background-color: #e9ecef; padding: 4px; border-radius: 3px; }}
    </style>
    <script>
        function deleteInstance(id, name) {{
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
    </div>

    <h1>Manage Instances{}</h1>

    <div class="controls">
        <form method="get" style="display: flex; gap: 10px; align-items: center;">
            <input type="text" name="search" placeholder="Search instances..." value="{}" />
            <input type="hidden" name="page" value="1" />
            <input type="hidden" name="per_page" value="{}" />
            <button type="submit" class="btn">Search</button>
        </form>
        {}
    </div>

    <p>Showing {} instances (Page {} of {})</p>

    <table>
        <thead>
            <tr>
                <th>{}</th>
                <th>{}</th>
                <th>{}</th>
                <th>{}</th>
                <th>{}</th>
                <th>{}</th>
                <th>{}</th>
                <th>{}</th>
                <th>Notes</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
"#,
        // Filter indicator for title
        if verified.is_some() {
            let mut filters = Vec::new();
            if let Some(ref v) = verified {
                filters.push(if v == "true" {
                    "Verified"
                } else {
                    "Unverified"
                });
            }
            format!(" - {}", filters.join(", "))
        } else {
            String::new()
        },
        search,
        per_page,
        // Clear filters link
        if verified.is_some() {
            r#"<a href="/admin/instances" class="btn" style="background: #6c757d;">Clear Filters</a>"#
        } else {
            ""
        },
        total_count,
        page,
        total_pages,
        generate_sortable_header(
            "id",
            "ID",
            &params.sort,
            &params.order,
            "/admin/instances",
            &search,
        ),
        generate_sortable_header(
            "name",
            "Name",
            &params.sort,
            &params.order,
            "/admin/instances",
            &search,
        ),
        generate_sortable_header(
            "expansion",
            "Expansion",
            &params.sort,
            &params.order,
            "/admin/instances",
            &search,
        ),
        generate_sortable_header(
            "zone_type",
            "Zone Type",
            &params.sort,
            &params.order,
            "/admin/instances",
            &search,
        ),
        generate_sortable_header(
            "level_ranges",
            "Level Ranges",
            &params.sort,
            &params.order,
            "/admin/instances",
            &search,
        ),
        generate_sortable_header(
            "rating",
            "Rating",
            &params.sort,
            &params.order,
            "/admin/instances",
            &search,
        ),
        generate_sortable_header(
            "hot_zone",
            "Hot Zone",
            &params.sort,
            &params.order,
            "/admin/instances",
            &search,
        ),
        generate_sortable_header(
            "verified",
            "Verified",
            &params.sort,
            &params.order,
            "/admin/instances",
            &search,
        ),
    );

    for instance in instances {
        html.push_str(&format!(r#"
            <tr>
                <td>{}</td>
                <td class="truncate" title="{}">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td class="truncate">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>
                    <a href="/admin/instances/{}" class="btn btn-small">Edit</a>
                    <a href="/admin/instances/{}/notes" class="btn btn-small">Notes</a>
                    <form id="delete-form-{}" method="post" action="/admin/instances/{}" style="display: inline;">
                        <input type="hidden" name="_method" value="DELETE" />
                        <button type="button" onclick="deleteInstance({}, '{}')" class="btn btn-danger btn-small">Delete</button>
                    </form>
                </td>
            </tr>
        "#,
            instance.id.unwrap_or(0),
            instance.name,
            instance.name,
            instance.expansion,
            instance.zone_type,
            instance.level_ranges,
            instance.rating,
            if instance.hot_zone { "✓" } else { "✗" },
            if instance.verified { "✓" } else { "✗" },
            instance.notes.len(),
            instance.id.unwrap_or(0),
            instance.id.unwrap_or(0),
            instance.id.unwrap_or(0),
            instance.id.unwrap_or(0),
            instance.id.unwrap_or(0),
            instance.name.replace("'", "\\'")
        ));
    }

    html.push_str("</tbody></table>");

    // Pagination
    if total_pages > 1 {
        html.push_str("<div class=\"pagination\">");

        let mut params = Vec::new();

        if !search.is_empty() {
            params.push(format!("search={}", urlencoding::encode(&search)));
        }

        if let Some(ref v) = verified {
            params.push(format!("verified={}", urlencoding::encode(v)));
        }

        params.push(format!("sort={}", urlencoding::encode(&sort)));
        params.push(format!("order={}", urlencoding::encode(&order)));

        let param_string = if params.is_empty() {
            String::new()
        } else {
            format!("&{}", params.join("&"))
        };

        if page > 1 {
            html.push_str(&format!(
                r#"<a href="?page={}&per_page={}{}">Previous</a>"#,
                page - 1,
                per_page,
                param_string
            ));
        }

        for p in 1..=total_pages {
            if p == page {
                html.push_str(&format!("<a href=\"#\" class=\"current\">{}</a>", p));
            } else {
                html.push_str(&format!(
                    "<a href=\"?page={}&per_page={}{}\">{}</a>",
                    p, per_page, param_string, p
                ));
            }
        }

        if page < total_pages {
            html.push_str(&format!(
                r#"<a href="?page={}&per_page={}{}">Next</a>"#,
                page + 1,
                per_page,
                param_string
            ));
        }

        html.push_str("</div>");
    }

    html.push_str("</body></html>");

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn edit_instance_form(
    State(state): State<AppState>,
    Path(instance_id): Path<i32>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.instance_state.pool;

    let instance_row = sqlx::query("SELECT id, name, level_ranges, expansion, continent, zone_type, connections, image_url, map_url, rating, hot_zone, verified FROM instances WHERE id = ?")
        .bind(instance_id)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let header = get_instance_form_header("Edit Instance");
    let body = get_instance_form_body(&instance_row, Some(instance_id));

    Ok(Html(format!("{}{}", header, body)))
}

#[cfg(feature = "admin")]
pub async fn handle_instance_update_or_delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<InstanceForm>,
) -> Result<Redirect, StatusCode> {
    if form._method.as_deref() == Some("DELETE") {
        delete_instance(State(state), Path(id)).await
    } else {
        update_instance(State(state), Path(id), Form(form)).await
    }
}

#[cfg(feature = "admin")]
pub async fn update_instance(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<InstanceForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.instance_state.pool;

    let hot_zone = form.hot_zone.is_some();
    let verified = form.verified.is_some();

    let _ = sqlx::query(
        "UPDATE instances SET name = ?, level_ranges = ?, expansion = ?, continent = ?, zone_type = ?, connections = ?, image_url = ?, map_url = ?, rating = ?, hot_zone = ?, verified = ? WHERE id = ?",
    )
    .bind(&form.name)
    .bind(&form.level_ranges)
    .bind(&form.expansion)
    .bind(&form.continent)
    .bind(&form.zone_type)
    .bind(&form.connections)
    .bind(&form.image_url)
    .bind(&form.map_url)
    .bind(form.rating)
    .bind(hot_zone)
    .bind(verified)
    .bind(id)
    .execute(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to("/admin/instances"))
}

#[cfg(feature = "admin")]
pub async fn delete_instance(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.instance_state.pool;

    let _ = sqlx::query("DELETE FROM instances WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to("/admin/instances"))
}

#[cfg(feature = "admin")]
pub async fn instance_notes(
    State(state): State<AppState>,
    Path(instance_id): Path<i32>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.instance_state.pool;

    // Get instance details
    let instance_row = sqlx::query("SELECT name FROM instances WHERE id = ?")
        .bind(instance_id)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let instance_name: String = instance_row.get("name");

    // Get notes for this instance
    let notes = crate::instances::get_instance_notes(pool.as_ref(), instance_id as i64)
        .await
        .unwrap_or_default();

    // Get note types for the form
    let note_types = crate::instances::get_note_types(pool.as_ref())
        .await
        .unwrap_or_default();

    let mut html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Instance Notes - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .form-group {{ margin-bottom: 15px; }}
        label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        input, textarea, select {{ width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }}
        textarea {{ height: 80px; resize: vertical; }}
        .btn {{ background: #007bff; color: white; padding: 8px 15px; border: none; border-radius: 4px; cursor: pointer; text-decoration: none; display: inline-block; }}
        .btn:hover {{ background: #0056b3; }}
        .btn-danger {{ background: #dc3545; }}
        .btn-danger:hover {{ background: #c82333; }}
        .btn-small {{ padding: 4px 8px; font-size: 0.8em; }}
        .note-card {{ border: 1px solid #ddd; padding: 15px; margin-bottom: 15px; border-radius: 5px; }}
        .note-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }}
        .note-type {{ padding: 4px 8px; border-radius: 3px; color: white; font-size: 0.8em; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/instances">Manage Instances</a>
        <a href="/admin/instances/{}/notes">Instance Notes</a>
    </div>

    <h1>Notes for: {}</h1>

    <div class="note-card">
        <h3>Add New Note</h3>
        <form method="post" action="/admin/instances/{}/notes">
            <div class="form-group">
                <label for="note_type_id">Note Type:</label>
                <select id="note_type_id" name="note_type_id" required>
"#,
        instance_id, instance_name, instance_id
    );

    for note_type in &note_types {
        html.push_str(&format!(
            r#"                    <option value="{}">{}</option>
"#,
            note_type.id.unwrap_or(0),
            note_type.display_name
        ));
    }

    html.push_str(
        r#"                </select>
            </div>
            <div class="form-group">
                <label for="content">Content:</label>
                <textarea id="content" name="content" required></textarea>
            </div>
            <button type="submit" class="btn">Add Note</button>
        </form>
    </div>

    <h2>Existing Notes</h2>
"#,
    );

    if notes.is_empty() {
        html.push_str("<p>No notes found for this instance.</p>");
    } else {
        for note in notes {
            html.push_str(&format!(
                r#"
    <div class="note-card">
        <div class="note-header">
            <span class="note-type" style="background: #007bff;">{}</span>
            <form method="post" action="/admin/instances/{}/notes/{}/delete" style="display: inline;">
                <button type="submit" class="btn btn-danger btn-small" onclick="return confirm('Delete this note?')">Delete</button>
            </form>
        </div>
        <p>{}</p>
    </div>
"#,
                note.note_type
                    .as_ref()
                    .map(|nt| nt.display_name.as_str())
                    .unwrap_or("Unknown"),
                instance_id,
                note.id.unwrap_or(0),
                note.content
            ));
        }
    }

    html.push_str("</body></html>");

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn create_instance_note(
    State(state): State<AppState>,
    Path(instance_id): Path<i32>,
    Form(form): Form<InstanceNoteForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.instance_state.pool;

    let sanitized_content = security::sanitize_user_input_with_formatting(&form.content);
    if sanitized_content != form.content {
        tracing::warn!(
            instance_id,
            "sanitized instance note content differed from input"
        );
    }

    let _ = sqlx::query(
        "INSERT INTO instance_notes (instance_id, note_type_id, content) VALUES (?, ?, ?)",
    )
    .bind(instance_id)
    .bind(form.note_type_id)
    .bind(&sanitized_content)
    .execute(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to(&format!(
        "/admin/instances/{}/notes",
        instance_id
    )))
}

#[cfg(feature = "admin")]
pub async fn delete_instance_note(
    State(state): State<AppState>,
    Path((instance_id, note_id)): Path<(i32, i32)>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.instance_state.pool;

    let _ = sqlx::query("DELETE FROM instance_notes WHERE id = ? AND instance_id = ?")
        .bind(note_id)
        .bind(instance_id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to(&format!(
        "/admin/instances/{}/notes",
        instance_id
    )))
}

#[cfg(feature = "admin")]
fn get_instance_form_header(title: &str) -> String {
    format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>{} - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .form-group {{ margin-bottom: 15px; }}
        label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        input, textarea, select {{ width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }}
        textarea {{ height: 60px; resize: vertical; }}
        .btn {{ background: #007bff; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; }}
        .btn:hover {{ background: #0056b3; }}
        .checkbox-group {{ display: flex; align-items: center; }}
        .checkbox-group input {{ width: auto; margin-right: 10px; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/instances">Manage Instances</a>
    </div>

    <h1>{}</h1>
"#,
        title, title
    )
}

#[cfg(feature = "admin")]
fn get_instance_form_body(
    instance_row: &sqlx::sqlite::SqliteRow,
    instance_id: Option<i32>,
) -> String {
    use sqlx::Row;

    let action = if let Some(id) = instance_id {
        format!("/admin/instances/{}", id)
    } else {
        "/admin/instances".to_string()
    };

    let method_field = if instance_id.is_some() {
        r#"<input type="hidden" name="_method" value="PUT">"#
    } else {
        ""
    };

    format!(
        r#"
    <form method="post" action="{}">
        {}
        <div class="form-group">
            <label for="name">Name:</label>
            <input type="text" id="name" name="name" value="{}" required>
        </div>

        <div class="form-group">
            <label for="level_ranges">Level Ranges (JSON):</label>
            <textarea id="level_ranges" name="level_ranges" required>{}</textarea>
        </div>

        <div class="form-group">
            <label for="expansion">Expansion:</label>
            <input type="text" id="expansion" name="expansion" value="{}" required>
        </div>

        <div class="form-group">
            <label for="continent">Continent:</label>
            <input type="text" id="continent" name="continent" value="{}">
        </div>

        <div class="form-group">
            <label for="zone_type">Instance Type:</label>
            <input type="text" id="zone_type" name="zone_type" value="{}" required>
        </div>

        <div class="form-group">
            <label for="connections">Connections (JSON):</label>
            <textarea id="connections" name="connections">{}</textarea>
        </div>

        <div class="form-group">
            <label for="image_url">Image URL:</label>
            <input type="url" id="image_url" name="image_url" value="{}">
        </div>

        <div class="form-group">
            <label for="map_url">Map URL:</label>
            <input type="url" id="map_url" name="map_url" value="{}">
        </div>

        <div class="form-group">
            <label for="rating">Rating (0-5):</label>
            <input type="number" id="rating" name="rating" min="0" max="5" value="{}">
        </div>

        <div class="form-group checkbox-group">
            <input type="checkbox" id="hot_zone" name="hot_zone" value="true" {}>
            <label for="hot_zone">Hot Zone</label>
        </div>

        <div class="form-group checkbox-group">
            <input type="checkbox" id="verified" name="verified" value="true" {}>
            <label for="verified">Verified</label>
        </div>

        <button type="submit" class="btn">Save Instance</button>
        <a href="/admin/instances" class="btn" style="background: #6c757d; margin-left: 10px;">Cancel</a>
    </form>

</body>
</html>
"#,
        action,
        method_field,
        instance_row.get::<String, _>("name"),
        instance_row.get::<String, _>("level_ranges"),
        instance_row.get::<String, _>("expansion"),
        instance_row.get::<String, _>("continent"),
        instance_row.get::<String, _>("zone_type"),
        instance_row.get::<String, _>("connections"),
        instance_row.get::<String, _>("image_url"),
        instance_row.get::<String, _>("map_url"),
        instance_row.get::<i32, _>("rating"),
        if instance_row.get::<bool, _>("hot_zone") {
            "checked"
        } else {
            ""
        },
        if instance_row.get::<bool, _>("verified") {
            "checked"
        } else {
            ""
        }
    )
}

#[cfg(all(test, feature = "admin"))]
mod tests {
    use super::*;
    use axum::extract::{Path, State};
    use axum::Form;
    use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::admin::types::InstanceNoteForm;
    use crate::classes::ClassRaceState;
    use crate::config::{
        AdminConfig, AppConfig, CorsConfig, DatabaseConfig, LoggingConfig, RatingsConfig,
        SecurityConfig, ServerConfig,
    };
    use crate::instances::InstanceState;
    use crate::zones::ZoneState;

    fn test_config() -> AppConfig {
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
                rating_ip_hash_key: "test".to_string(),
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

        sqlx::query(
            r#"CREATE TABLE instance_notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                instance_id INTEGER NOT NULL,
                note_type_id INTEGER NOT NULL,
                content TEXT NOT NULL
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let pool_arc = Arc::new(pool);
        let state = AppState {
            config: Arc::new(test_config()),
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

    #[tokio::test]
    async fn create_instance_note_sanitizes_script_tags() {
        let (state, pool) = setup_state().await;
        let raw = "<script>alert('y')</script>";
        let form = InstanceNoteForm {
            note_type_id: 1,
            content: raw.to_string(),
        };

        let _ = create_instance_note(State(state), Path(1), Form(form))
            .await
            .unwrap();

        let row = sqlx::query("SELECT content FROM instance_notes WHERE instance_id = ?")
            .bind(1)
            .fetch_one(&*pool)
            .await
            .unwrap();
        let stored: String = row.get("content");
        let expected = crate::security::sanitize_user_input_with_formatting(raw);
        assert_eq!(stored, expected);
    }
}
