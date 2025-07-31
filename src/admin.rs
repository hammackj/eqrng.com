use axum::Router;

#[cfg(feature = "admin")]
use axum::{
    Form,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
    routing::{get, post},
};
#[cfg(feature = "admin")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "admin")]
use sqlx::Row;
#[cfg(feature = "admin")]
use std::collections::HashMap;
#[cfg(feature = "admin")]
use urlencoding;

#[cfg(not(feature = "admin"))]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::AppState;

#[cfg(feature = "admin")]
#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    pub id: Option<i32>,
    pub name: String,
    pub level_ranges: String, // JSON string
    pub expansion: String,
    pub continent: String,
    pub zone_type: String,
    pub connections: String, // JSON string
    pub image_url: String,
    pub map_url: String,
    pub rating: i32,
    pub hot_zone: bool,
    pub mission: bool,
    pub verified: bool,
    pub notes: Vec<crate::zones::ZoneNote>,
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct ZoneForm {
    pub name: String,
    pub level_ranges: String,
    pub expansion: String,
    pub continent: String,
    pub zone_type: String,
    pub connections: String,
    pub image_url: String,
    pub map_url: String,
    pub rating: i32,
    pub hot_zone: Option<String>, // HTML forms send "on" or nothing
    pub mission: Option<String>,
    pub verified: Option<String>, // HTML forms send "on" or nothing
    pub _method: Option<String>,  // For method override
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
    pub search: Option<String>,
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct ZoneNoteForm {
    pub note_type_id: i64,
    pub content: String,
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct NoteTypeForm {
    pub name: String,
    pub display_name: String,
    pub color_class: String,
}

#[cfg(feature = "admin")]
#[derive(Deserialize)]
pub struct LinkForm {
    pub name: String,
    pub url: String,
    pub category: String,
    pub description: Option<String>,
    pub _method: Option<String>,
}

#[cfg(feature = "admin")]
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/admin", get(admin_dashboard))
        .route("/admin/zones", get(list_zones))
        .route("/admin/zones/new", get(new_zone_form))
        .route("/admin/zones", post(create_zone))
        .route("/admin/zones/:id", get(edit_zone_form))
        .route("/admin/zones/:id", post(handle_zone_update_or_delete))
        .route("/admin/zones/:id/delete", post(delete_zone))
        .route("/admin/zones/:id/ratings", get(zone_ratings))
        .route("/admin/zones/:id/notes", get(zone_notes))
        .route("/admin/zones/:id/notes", post(create_zone_note))
        .route(
            "/admin/zones/:id/notes/:note_id/delete",
            post(delete_zone_note),
        )
        .route("/admin/note-types", get(list_note_types))
        .route("/admin/note-types", post(create_note_type))
        .route("/admin/note-types/:id/delete", post(delete_note_type))
        .route("/admin/ratings", get(list_all_ratings))
        .route("/admin/ratings/:id/delete", post(handle_rating_delete))
        .route("/admin/links", get(list_links))
        .route("/admin/links/new", get(new_link_form))
        .route("/admin/links", post(create_link_admin))
        .route("/admin/links/:id", get(edit_link_form))
        .route("/admin/links/:id", post(handle_link_update_or_delete))
        .route("/admin/links/:id/delete", post(delete_link_admin))
}

#[cfg(not(feature = "admin"))]
pub fn admin_routes() -> Router<AppState> {
    Router::new()
}

#[cfg(feature = "admin")]
async fn admin_dashboard(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get statistics
    let zone_count: i32 = sqlx::query("SELECT COUNT(*) as count FROM zones")
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get("count");

    let rating_count: i32 = sqlx::query("SELECT COUNT(*) as count FROM zone_ratings")
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get("count");

    let avg_rating: Option<f64> = sqlx::query("SELECT AVG(rating) as avg_rating FROM zone_ratings")
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get("avg_rating");

    let hot_zone_count: i32 = sqlx::query("SELECT COUNT(*) as count FROM zones WHERE hot_zone = 1")
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get("count");

    let avg_rating_display = avg_rating.map_or("N/A".to_string(), |r| format!("{:.1}", r));

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .card {{ border: 1px solid #ddd; padding: 20px; margin-bottom: 20px; border-radius: 5px; }}
        .stats {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin-bottom: 20px; }}
        .stat-card {{ background: #f8f9fa; padding: 20px; text-align: center; border-radius: 5px; }}
        .stat-number {{ font-size: 2em; font-weight: bold; color: #007bff; }}
        .stat-label {{ font-size: 14px; color: #666; margin-top: 5px; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/zones/new">Add New Zone</a>
        <a href="/admin/note-types">Manage Note Types</a>
        <a href="/admin/ratings">Manage Ratings</a>
        <a href="/admin/links">Manage Links</a>
    </div>

    <h1>EQ RNG Admin Dashboard</h1>

    <div class="stats">
        <div class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Total Zones</div>
        </div>
        <div class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Total Ratings</div>
        </div>
        <div class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Average Rating</div>
        </div>
        <div class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Hot Zones</div>
        </div>
    </div>

    <div class="card">
        <h2>Quick Actions</h2>
        <p><a href="/admin/zones">Manage all zones</a> - View, edit, and delete zones</p>
        <p><a href="/admin/zones/new">Add new zone</a> - Create a new zone entry</p>
        <p><a href="/admin/note-types">Manage note types</a> - Configure pill icons for zone notes</p>
        <p><a href="/admin/ratings">Manage all ratings</a> - View and delete zone ratings</p>
        <p><a href="/admin/links">Manage links</a> - View, edit, and delete links organized by category</p>
    </div>

    <div class="card">
        <h2>System Status</h2>
        <p>Admin interface is running in development mode.</p>
        <p><strong>Warning:</strong> This interface should only be used locally for development.</p>
    </div>
</body>
</html>
    "#,
        zone_count, rating_count, avg_rating_display, hot_zone_count
    );
    Ok(Html(html))
}

#[cfg(feature = "admin")]
async fn list_zones(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Html<String>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(5, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.unwrap_or_default();

    let pool = &state.zone_state.pool;

    // Build search query
    let (where_clause, search_param) = if search.is_empty() {
        ("".to_string(), None)
    } else {
        (
            "WHERE name LIKE ? OR expansion LIKE ? OR zone_type LIKE ?".to_string(),
            Some(format!("%{}%", search)),
        )
    };

    // Get total count
    let count_query = format!("SELECT COUNT(*) as count FROM zones {}", where_clause);
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

    // Get zones
    let zones_query = format!(
        "SELECT * FROM zones {} ORDER BY name LIMIT ? OFFSET ?",
        where_clause
    );

    let zone_rows = if let Some(ref search_term) = search_param {
        sqlx::query(&zones_query)
            .bind(search_term)
            .bind(search_term)
            .bind(search_term)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        sqlx::query(&zones_query)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    // Load notes for each zone
    let mut zones = Vec::new();
    for row in zone_rows {
        let zone_id: i64 = row.get("id");
        let notes = crate::zones::get_zone_notes(pool.as_ref(), zone_id)
            .await
            .unwrap_or_default();

        zones.push(Zone {
            id: Some(zone_id as i32),
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
            mission: row.get("mission"),
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
    <title>Manage Zones - EQ RNG Admin</title>
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
    </style>
    <script>
        function deleteZone(id, name) {{
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
        <a href="/admin/zones/new">Add New Zone</a>
    </div>

    <h1>Manage Zones</h1>

    <div class="controls">
        <form method="get" style="display: flex; gap: 10px; align-items: center;">
            <input type="text" name="search" placeholder="Search zones..." value="{}" />
            <input type="hidden" name="page" value="1" />
            <input type="hidden" name="per_page" value="{}" />
            <button type="submit" class="btn">Search</button>
        </form>
        <a href="/admin/zones/new" class="btn">Add New Zone</a>
    </div>

    <p>Showing {} zones (Page {} of {})</p>

    <table>
        <thead>
            <tr>
                <th>ID</th>
                <th>Name</th>
                <th>Expansion</th>
                <th>Zone Type</th>
                <th>Level Ranges</th>
                <th>Rating</th>
                <th>Hot Zone</th>
                <th>Mission</th>
                <th>Verified</th>
                <th>Notes</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
"#,
        search, per_page, total_count, page, total_pages
    );

    for zone in zones {
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
                <td>{}</td>
                <td>
                    <a href="/admin/zones/{}" class="btn btn-small">Edit</a>
                    <a href="/admin/zones/{}/ratings" class="btn btn-small">Ratings</a>
                    <a href="/admin/zones/{}/notes" class="btn btn-small">Notes</a>
                    <form id="delete-form-{}" method="post" action="/admin/zones/{}" style="display: inline;">
                        <input type="hidden" name="_method" value="DELETE" />
                        <button type="button" onclick="deleteZone({}, '{}')" class="btn btn-danger btn-small">Delete</button>
                    </form>
                </td>
            </tr>
        "#,
            zone.id.unwrap_or(0),
            zone.name, zone.name,
            zone.expansion,
            zone.zone_type,
            zone.level_ranges,
            zone.rating,
            if zone.hot_zone { "✓" } else { "✗" },
            if zone.mission { "✓" } else { "✗" },
            if zone.verified { "✓" } else { "✗" },
            zone.notes.len(),
            zone.id.unwrap_or(0),
            zone.id.unwrap_or(0),
            zone.id.unwrap_or(0),
            zone.id.unwrap_or(0),
            zone.id.unwrap_or(0),
            zone.id.unwrap_or(0),
            zone.name.replace("'", "\\'")
        ));
    }

    html.push_str("</tbody></table>");

    // Pagination
    if total_pages > 1 {
        html.push_str("<div class=\"pagination\">");

        if page > 1 {
            html.push_str(&format!(
                r#"<a href="?page={}&per_page={}&search={}">Previous</a>"#,
                page - 1,
                per_page,
                search
            ));
        }

        for p in 1..=total_pages {
            if p == page {
                html.push_str(&format!("<a href=\"#\" class=\"current\">{}</a>", p));
            } else {
                html.push_str(&format!(
                    "<a href=\"?page={}&per_page={}&search={}\">{}</a>",
                    p, per_page, search, p
                ));
            }
        }

        if page < total_pages {
            html.push_str(&format!(
                r#"<a href="?page={}&per_page={}&search={}">Next</a>"#,
                page + 1,
                per_page,
                search
            ));
        }

        html.push_str("</div>");
    }

    html.push_str("</body></html>");

    Ok(Html(html))
}

#[cfg(feature = "admin")]
async fn new_zone_form() -> Html<String> {
    let html = format!(
        "{}{}",
        get_zone_form_header(),
        get_zone_form_body(
            "",
            &Zone {
                notes: Vec::new(),
                id: None,
                name: String::new(),
                level_ranges: "[]".to_string(),
                expansion: String::new(),
                continent: String::new(),
                zone_type: String::new(),
                connections: "[]".to_string(),
                image_url: String::new(),
                map_url: String::new(),
                rating: 0,
                hot_zone: false,
                mission: false,
                verified: false,
            },
            "/admin/zones",
            "POST",
            "Create Zone"
        )
    );

    Html(html)
}

#[cfg(feature = "admin")]
async fn edit_zone_form(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    let zone_row = sqlx::query("SELECT * FROM zones WHERE id = ?")
        .bind(id)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let zone = Zone {
        id: Some(id),
        name: zone_row.get("name"),
        level_ranges: zone_row.get("level_ranges"),
        expansion: zone_row.get("expansion"),
        continent: zone_row.get("continent"),
        zone_type: zone_row.get("zone_type"),
        connections: zone_row.get("connections"),
        image_url: zone_row.get("image_url"),
        map_url: zone_row.get("map_url"),
        rating: zone_row.get("rating"),
        hot_zone: zone_row.get("hot_zone"),
        mission: zone_row.get("mission"),
        verified: zone_row.get("verified"),
        notes: Vec::new(),
    };

    let html = format!(
        "{}{}",
        get_zone_form_header(),
        get_zone_form_body(
            &format!("Edit Zone: {}", zone.name),
            &zone,
            &format!("/admin/zones/{}", id),
            "PUT",
            "Update Zone"
        )
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
fn get_zone_form_header() -> String {
    r#"
<!DOCTYPE html>
<html>
<head>
    <title>Zone Form - EQ RNG Admin</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        .nav { background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }
        .nav a { margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }
        .nav a:hover { color: #007bff; }
        .form-group { margin-bottom: 15px; }
        label { display: block; margin-bottom: 5px; font-weight: bold; }
        input, textarea, select { width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }
        textarea { height: 80px; resize: vertical; }
        .checkbox-group { display: flex; align-items: center; gap: 10px; }
        .checkbox-group input { width: auto; }
        .btn { background: #007bff; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; }
        .btn:hover { background: #0056b3; }
        .btn-secondary { background: #6c757d; margin-left: 10px; }
        .btn-secondary:hover { background: #545b62; }
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/zones/new">Add New Zone</a>
    </div>
"#.to_string()
}

#[cfg(feature = "admin")]
fn get_zone_form_body(
    title: &str,
    zone: &Zone,
    action: &str,
    method: &str,
    button_text: &str,
) -> String {
    format!(
        r#"
    <h1>{}</h1>

    <form action="{}" method="post">
        <input type="hidden" name="_method" value="{}" />

        <div class="form-group">
            <label for="name">Name *</label>
            <input type="text" id="name" name="name" value="{}" required />
        </div>

        <div class="form-group">
            <label for="expansion">Expansion *</label>
            <input type="text" id="expansion" name="expansion" value="{}" required />
        </div>

        <div class="form-group">
            <label for="continent">Continent</label>
            <input type="text" id="continent" name="continent" value="{}" />
        </div>

        <div class="form-group">
            <label for="zone_type">Zone Type *</label>
            <select id="zone_type" name="zone_type" required>
                <option value="">Select type...</option>
                <option value="outdoor" {}>Outdoor</option>
                <option value="dungeon" {}>Dungeon</option>
                <option value="raid" {}>Raid</option>
                <option value="city" {}>City</option>
                <option value="instanced" {}>Instanced</option>
                <option value="mission" {}>Mission</option>
            </select>
        </div>

        <div class="form-group">
            <label for="level_ranges">Level Ranges (JSON format, e.g., [[1,10],[15,20]]) *</label>
            <textarea id="level_ranges" name="level_ranges" required>{}</textarea>
        </div>

        <div class="form-group">
            <label for="connections">Connections (JSON array, e.g., ["zone1","zone2"]) *</label>
            <textarea id="connections" name="connections" required>{}</textarea>
        </div>

        <div class="form-group">
            <label for="image_url">Image URL</label>
            <input type="url" id="image_url" name="image_url" value="{}" />
        </div>

        <div class="form-group">
            <label for="map_url">Map URL</label>
            <input type="url" id="map_url" name="map_url" value="{}" />
        </div>

        <div class="form-group">
            <label for="rating">Default Rating (0-5)</label>
            <input type="number" id="rating" name="rating" value="{}" min="0" max="5" />
        </div>

        <div class="form-group checkbox-group">
            <input type="checkbox" id="hot_zone" name="hot_zone" {} />
            <label for="hot_zone">Hot Zone</label>
        </div>

        <div class="form-group checkbox-group">
            <input type="checkbox" id="mission" name="mission" {} />
            <label for="mission">Mission Zone</label>
        </div>

        <div class="form-group checkbox-group">
            <input type="checkbox" id="verified" name="verified" {} />
            <label for="verified">Verified</label>
        </div>

        <button type="submit" class="btn">{}</button>
        <a href="/admin/zones" class="btn btn-secondary">Cancel</a>
    </form>
</body>
</html>
    "#,
        title,
        action,
        method,
        zone.name,
        zone.expansion,
        zone.continent,
        if zone.zone_type == "outdoor" {
            "selected"
        } else {
            ""
        },
        if zone.zone_type == "dungeon" {
            "selected"
        } else {
            ""
        },
        if zone.zone_type == "raid" {
            "selected"
        } else {
            ""
        },
        if zone.zone_type == "city" {
            "selected"
        } else {
            ""
        },
        if zone.zone_type == "instanced" {
            "selected"
        } else {
            ""
        },
        if zone.zone_type == "mission" {
            "selected"
        } else {
            ""
        },
        zone.level_ranges,
        zone.connections,
        zone.image_url,
        zone.map_url,
        zone.rating,
        if zone.hot_zone { "checked" } else { "" },
        if zone.mission { "checked" } else { "" },
        if zone.verified { "checked" } else { "" },
        button_text
    )
}

#[cfg(feature = "admin")]
async fn create_zone(
    State(state): State<AppState>,
    Form(form): Form<ZoneForm>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Validate JSON fields
    if serde_json::from_str::<serde_json::Value>(&form.level_ranges).is_err() {
        return Ok(Html(format!(
            r#"<h1>Error</h1><p>Invalid level_ranges JSON format</p><a href="/admin/zones/new">Go back</a>"#
        )));
    }

    if serde_json::from_str::<serde_json::Value>(&form.connections).is_err() {
        return Ok(Html(format!(
            r#"<h1>Error</h1><p>Invalid connections JSON format</p><a href="/admin/zones/new">Go back</a>"#
        )));
    }

    let result = sqlx::query(
        r#"
        INSERT INTO zones (
            name, level_ranges, expansion, continent, zone_type,
            connections, image_url, map_url, rating, hot_zone, mission, verified
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
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
    .bind(form.hot_zone.is_some())
    .bind(form.mission.is_some())
    .bind(form.verified.is_some())
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => {
            // Force WAL checkpoint to immediately update main database file
            let _ = crate::checkpoint_wal(pool.as_ref()).await;

            Ok(Html(format!(
                r#"<h1>Success</h1><p>Zone "{}" created successfully!</p><a href="/admin/zones">Back to zones</a>"#,
                form.name
            )))
        }
        Err(_) => Ok(Html(format!(
            r#"<h1>Error</h1><p>Failed to create zone</p><a href="/admin/zones/new">Go back</a>"#
        ))),
    }
}

#[cfg(feature = "admin")]
async fn update_zone(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<ZoneForm>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Validate JSON fields
    if serde_json::from_str::<serde_json::Value>(&form.level_ranges).is_err() {
        return Ok(Html(format!(
            r#"<h1>Error</h1><p>Invalid level_ranges JSON format</p><a href="/admin/zones/{}">Go back</a>"#,
            id
        )));
    }

    if serde_json::from_str::<serde_json::Value>(&form.connections).is_err() {
        return Ok(Html(format!(
            r#"<h1>Error</h1><p>Invalid connections JSON format</p><a href="/admin/zones/{}">Go back</a>"#,
            id
        )));
    }

    let result = sqlx::query(
        r#"
        UPDATE zones SET
            name = ?, level_ranges = ?, expansion = ?, continent = ?, zone_type = ?,
            connections = ?, image_url = ?, map_url = ?, rating = ?, hot_zone = ?, mission = ?, verified = ?
        WHERE id = ?
        "#,
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
    .bind(form.hot_zone.is_some())
    .bind(form.mission.is_some())
    .bind(form.verified.is_some())
    .bind(id)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => {
            // Force WAL checkpoint to immediately update main database file
            let _ = crate::checkpoint_wal(pool.as_ref()).await;

            Ok(Html(format!(
                r#"<h1>Success</h1><p>Zone "{}" updated successfully!</p><a href="/admin/zones">Back to zones</a>"#,
                form.name
            )))
        }
        Err(_) => Ok(Html(format!(
            r#"<h1>Error</h1><p>Failed to update zone</p><a href="/admin/zones/{}">Go back</a>"#,
            id
        ))),
    }
}

#[cfg(feature = "admin")]
async fn delete_zone(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let pool = &state.zone_state.pool;

    sqlx::query("DELETE FROM zones WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(StatusCode::OK)
}

#[cfg(feature = "admin")]
async fn delete_rating(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let pool = &state.zone_state.pool;

    let _ = sqlx::query("DELETE FROM zone_ratings WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(StatusCode::OK)
}

#[cfg(feature = "admin")]
async fn delete_link_admin(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    let _ = sqlx::query("DELETE FROM links WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to("/admin/links"))
}

#[cfg(feature = "admin")]
async fn zone_ratings(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get zone info
    let zone_row = sqlx::query("SELECT name FROM zones WHERE id = ?")
        .bind(id)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let zone_name: String = zone_row.get("name");

    // Get ratings
    let ratings = sqlx::query(
        "SELECT rating, user_ip, created_at FROM zone_ratings WHERE zone_id = ? ORDER BY created_at DESC"
    )
    .bind(id)
    .fetch_all(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Zone Ratings - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        table {{ width: 100%; border-collapse: collapse; }}
        th, td {{ padding: 10px; border: 1px solid #ddd; text-align: left; }}
        th {{ background: #f5f5f5; }}
        .rating {{ font-weight: bold; color: #007bff; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/zones/new">Add New Zone</a>
    </div>

    <h1>Ratings for: {}</h1>

    <p><a href="/admin/zones">← Back to zones</a></p>

    <table>
        <thead>
            <tr>
                <th>Rating</th>
                <th>User IP (Anonymized)</th>
                <th>Date</th>
            </tr>
        </thead>
        <tbody>
    "#,
        zone_name
    );

    if ratings.is_empty() {
        html.push_str("<tr><td colspan=\"3\">No ratings yet</td></tr>");
    } else {
        for rating in ratings {
            let rating_value: i32 = rating.get("rating");
            let user_ip: String = rating.get("user_ip");
            let created_at: String = rating.get("created_at");

            // Anonymize IP (show only first two octets)
            let anonymized_ip = user_ip.split('.').take(2).collect::<Vec<_>>().join(".") + ".x.x";

            html.push_str(&format!(
                "<tr><td class=\"rating\">{}</td><td>{}</td><td>{}</td></tr>",
                rating_value, anonymized_ip, created_at
            ));
        }
    }

    html.push_str("</tbody></table></body></html>");

    Ok(Html(html))
}

#[cfg(feature = "admin")]
async fn handle_zone_update_or_delete(
    state: State<AppState>,
    path: Path<i32>,
    Form(form): Form<ZoneForm>,
) -> Result<Html<String>, StatusCode> {
    match form._method.as_deref() {
        Some("PUT") => update_zone(state, path, Form(form)).await,
        Some("DELETE") => {
            delete_zone(state, path).await?;
            Ok(Html(format!(
                r#"<h1>Success</h1><p>Zone deleted successfully!</p><a href="/admin/zones">Back to zones</a>"#
            )))
        }
        _ => update_zone(state, path, Form(form)).await,
    }
}

#[cfg(feature = "admin")]
async fn list_all_ratings(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Html<String>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(5, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.unwrap_or_default();

    let pool = &state.zone_state.pool;

    // Build search query
    let (where_clause, search_param) = if search.is_empty() {
        ("".to_string(), None)
    } else {
        (
            "WHERE z.name LIKE ? OR r.user_ip LIKE ?".to_string(),
            Some(format!("%{}%", search)),
        )
    };

    // Get total count
    let count_query = format!(
        "SELECT COUNT(*) as count FROM zone_ratings r JOIN zones z ON r.zone_id = z.id {}",
        where_clause
    );
    let total_count: i32 = if let Some(ref search_term) = search_param {
        sqlx::query(&count_query)
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

    // Get ratings with zone names
    let ratings_query = format!(
        "SELECT r.id, r.zone_id, z.name as zone_name, r.user_ip, r.rating, r.created_at, r.updated_at
         FROM zone_ratings r
         JOIN zones z ON r.zone_id = z.id
         {}
         ORDER BY r.created_at DESC
         LIMIT ? OFFSET ?",
        where_clause
    );

    let ratings_rows = if let Some(ref search_term) = search_param {
        sqlx::query(&ratings_query)
            .bind(search_term)
            .bind(search_term)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        sqlx::query(&ratings_query)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    let total_pages = (total_count + per_page - 1) / per_page;

    let mut html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Manage Ratings - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1400px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .search-bar {{ margin-bottom: 20px; }}
        .search-bar input {{ padding: 8px; width: 300px; border: 1px solid #ddd; border-radius: 4px; }}
        .search-bar button {{ padding: 8px 16px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; }}
        table {{ width: 100%; border-collapse: collapse; margin-bottom: 20px; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background-color: #f5f5f5; font-weight: bold; }}
        tr:hover {{ background-color: #f9f9f9; }}
        .pagination {{ text-align: center; margin-top: 20px; }}
        .pagination a {{ display: inline-block; padding: 8px 16px; margin: 0 4px; text-decoration: none; border: 1px solid #ddd; border-radius: 4px; }}
        .pagination a:hover {{ background-color: #f5f5f5; }}
        .pagination .current {{ background-color: #007bff; color: white; }}
        .btn {{ display: inline-block; padding: 6px 12px; text-decoration: none; border-radius: 4px; font-size: 12px; }}
        .btn-danger {{ background-color: #dc3545; color: white; border: none; cursor: pointer; }}
        .btn-danger:hover {{ background-color: #c82333; }}
        .rating-stars {{ color: #ffc107; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/zones/new">Add New Zone</a>
        <a href="/admin/ratings">Manage Ratings</a>
    </div>

    <h1>Manage Zone Ratings</h1>

    <div class="search-bar">
        <form method="get">
            <input type="text" name="search" placeholder="Search by zone name or IP..." value="{}" />
            <button type="submit">Search</button>
            {}"#,
        search,
        if !search.is_empty() {
            format!("&nbsp;<a href=\"/admin/ratings\">Clear</a>")
        } else {
            String::new()
        }
    );

    html.push_str("</form></div>");

    html.push_str(
        r#"
    <table>
        <thead>
            <tr>
                <th>ID</th>
                <th>Zone</th>
                <th>User IP</th>
                <th>Rating</th>
                <th>Created</th>
                <th>Updated</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>"#,
    );

    for row in ratings_rows {
        let id: i32 = row.get("id");
        let zone_name: String = row.get("zone_name");
        let user_ip: String = row.get("user_ip");
        let rating: i32 = row.get("rating");
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");

        let stars = "★".repeat(rating as usize) + &"☆".repeat(5 - rating as usize);

        html.push_str(&format!(
            r#"
            <tr>
                <td>{}</td>
                <td><a href="/admin/zones/{}/ratings">{}</a></td>
                <td>{}</td>
                <td><span class="rating-stars">{}</span> ({})</td>
                <td>{}</td>
                <td>{}</td>
                <td>
                    <form method="post" action="/admin/ratings/{}" style="display: inline;">
                        <input type="hidden" name="_method" value="DELETE">
                        <button type="submit" class="btn btn-danger" onclick="return confirm('Are you sure you want to delete this rating?')">Delete</button>
                    </form>
                </td>
            </tr>"#,
            id,
            row.get::<i32, _>("zone_id"),
            zone_name,
            user_ip,
            stars,
            rating,
            created_at,
            updated_at,
            id
        ));
    }

    html.push_str("</tbody></table>");

    // Pagination
    if total_pages > 1 {
        html.push_str("<div class=\"pagination\">");

        for p in 1..=total_pages {
            let search_param = if search.is_empty() {
                String::new()
            } else {
                format!("&search={}", urlencoding::encode(&search))
            };

            if p == page {
                html.push_str(&format!("<span class=\"pagination current\">{}</span>", p));
            } else {
                html.push_str(&format!(
                    "<a href=\"/admin/ratings?page={}{}\"> {} </a>",
                    p, search_param, p
                ));
            }
        }

        html.push_str("</div>");
    }

    html.push_str(&format!(
        r#"
    <div style="margin-top: 20px; padding: 15px; background-color: #f8f9fa; border-radius: 5px;">
        <strong>Total Ratings:</strong> {} | <strong>Page:</strong> {} of {} | <strong>Showing:</strong> {} per page
    </div>

</body>
</html>"#,
        total_count, page, total_pages, per_page
    ));

    Ok(Html(html))
}

#[cfg(feature = "admin")]
async fn zone_notes(
    Path(zone_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get zone info
    let zone_row = sqlx::query("SELECT name FROM zones WHERE id = ?")
        .bind(zone_id)
        .fetch_optional(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let zone_name: String = zone_row.get("name");

    // Get notes
    let notes = crate::zones::get_zone_notes(pool.as_ref(), zone_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get note types
    let note_types = crate::zones::get_note_types(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let notes_html = notes
        .iter()
        .map(|note| {
            let note_type = note.note_type.as_ref().unwrap();
            format!(
                r#"
                <div class="note-item" style="display: flex; align-items: center; margin-bottom: 10px; padding: 10px; background: #f8f9fa; border-radius: 5px;">
                    <span class="pill" style="background: {}; color: white; padding: 4px 8px; border-radius: 12px; font-size: 12px; margin-right: 10px;">{}</span>
                    <span style="flex: 1;">{}</span>
                    <form method="post" action="/admin/zones/{}/notes/{}" style="margin: 0;">
                        <input type="hidden" name="_method" value="delete">
                        <button type="submit" style="background: #dc3545; color: white; border: none; padding: 4px 8px; border-radius: 3px; cursor: pointer; font-size: 12px;">Delete</button>
                    </form>
                </div>
                "#,
                note_type.color_class.replace("bg-", "#").replace("-500", ""),
                note_type.display_name,
                note.content,
                zone_id,
                note.id.unwrap_or(0)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let note_type_options = note_types
        .iter()
        .map(|nt| {
            format!(
                r#"<option value="{}">{}</option>"#,
                nt.id.unwrap_or(0),
                nt.display_name
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Zone Notes - {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .form-group {{ margin-bottom: 15px; }}
        .form-group label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        .form-group input, .form-group select, .form-group textarea {{ width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; }}
        .btn {{ padding: 10px 15px; border: none; border-radius: 4px; cursor: pointer; text-decoration: none; display: inline-block; }}
        .btn-primary {{ background: #007bff; color: white; }}
        .btn-primary:hover {{ background: #0056b3; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/zones/new">Add New Zone</a>
        <a href="/admin/note-types">Manage Note Types</a>
        <a href="/admin/ratings">Manage Ratings</a>
    </div>

    <h1>Notes for Zone: {}</h1>

    <div style="margin-bottom: 30px;">
        <h2>Add New Note</h2>
        <form method="post" action="/admin/zones/{}/notes">
            <div class="form-group">
                <label>Note Type:</label>
                <select name="note_type_id" required>
                    <option value="">Select note type...</option>
                    {}
                </select>
            </div>
            <div class="form-group">
                <label>Content:</label>
                <input type="text" name="content" required placeholder="Enter note content... (HTML links supported: &lt;a href=&quot;url&quot;&gt;text&lt;/a&gt;)">
                <small style="color: #666; display: block; margin-top: 5px;">
                    Supports HTML: &lt;a href="https://example.com"&gt;Link Text&lt;/a&gt;, &lt;strong&gt;bold&lt;/strong&gt;, &lt;em&gt;italic&lt;/em&gt;
                </small>
            </div>
            <button type="submit" class="btn btn-primary">Add Note</button>
        </form>
    </div>

    <div>
        <h2>Existing Notes</h2>
        {}
        {}
    </div>

    <p><a href="/admin/zones">← Back to Zones</a></p>
</body>
</html>
        "#,
        zone_name,
        zone_name,
        zone_id,
        note_type_options,
        notes_html,
        if notes.is_empty() {
            "<p>No notes yet.</p>"
        } else {
            ""
        }
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
async fn create_zone_note(
    State(state): State<AppState>,
    Path(zone_id): Path<i32>,
    Form(form): Form<ZoneNoteForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    let _ = sqlx::query("INSERT INTO zone_notes (zone_id, note_type_id, content) VALUES (?, ?, ?)")
        .bind(zone_id)
        .bind(form.note_type_id)
        .bind(&form.content)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to(&format!("/admin/zones/{}/notes", zone_id)))
}

#[cfg(feature = "admin")]
async fn delete_zone_note(
    State(state): State<AppState>,
    Path((zone_id, note_id)): Path<(i32, i32)>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    let _ = sqlx::query("DELETE FROM zone_notes WHERE id = ? AND zone_id = ?")
        .bind(note_id)
        .bind(zone_id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to(&format!("/admin/zones/{}/notes", zone_id)))
}

#[cfg(feature = "admin")]
async fn list_note_types(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    let note_types = crate::zones::get_note_types(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let note_types_html = note_types
        .iter()
        .map(|nt| {
            format!(
                r#"
                <div style="display: flex; align-items: center; margin-bottom: 10px; padding: 15px; background: #f8f9fa; border-radius: 5px;">
                    <span class="pill" style="background: {}; color: white; padding: 6px 12px; border-radius: 15px; font-size: 14px; margin-right: 15px;">{}</span>
                    <div style="flex: 1;">
                        <strong>{}</strong><br>
                        <small style="color: #666;">Internal name: {}</small>
                    </div>
                    <form method="post" action="/admin/note-types/{}" style="margin: 0;">
                        <input type="hidden" name="_method" value="delete">
                        <button type="submit" style="background: #dc3545; color: white; border: none; padding: 8px 12px; border-radius: 4px; cursor: pointer;" onclick="return confirm('Are you sure?')">Delete</button>
                    </form>
                </div>
                "#,
                nt.color_class.replace("bg-", "#").replace("-500", ""),
                nt.display_name,
                nt.display_name,
                nt.name,
                nt.id.unwrap_or(0)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Manage Note Types</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .form-group {{ margin-bottom: 15px; }}
        .form-group label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        .form-group input, .form-group select {{ width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; }}
        .btn {{ padding: 10px 15px; border: none; border-radius: 4px; cursor: pointer; text-decoration: none; display: inline-block; }}
        .btn-primary {{ background: #007bff; color: white; }}
        .btn-primary:hover {{ background: #0056b3; }}
        .grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 30px; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/zones/new">Add New Zone</a>
        <a href="/admin/note-types">Manage Note Types</a>
        <a href="/admin/ratings">Manage Ratings</a>
    </div>

    <h1>Manage Note Types</h1>

    <div class="grid">
        <div>
            <h2>Add New Note Type</h2>
            <form method="post" action="/admin/note-types">
                <div class="form-group">
                    <label>Internal Name:</label>
                    <input type="text" name="name" required placeholder="epic_3_0" pattern="[a-z0-9_]+" title="Lowercase letters, numbers, and underscores only">
                </div>
                <div class="form-group">
                    <label>Display Name:</label>
                    <input type="text" name="display_name" required placeholder="Epic 3.0">
                </div>
                <div class="form-group">
                    <label>Color Class:</label>
                    <select name="color_class" required>
                        <option value="bg-blue-500">Blue</option>
                        <option value="bg-green-500">Green</option>
                        <option value="bg-yellow-500">Yellow</option>
                        <option value="bg-orange-500">Orange</option>
                        <option value="bg-red-500">Red</option>
                        <option value="bg-purple-500">Purple</option>
                        <option value="bg-pink-500">Pink</option>
                        <option value="bg-indigo-500">Indigo</option>
                    </select>
                </div>
                <button type="submit" class="btn btn-primary">Add Note Type</button>
            </form>
        </div>

        <div>
            <h2>Existing Note Types</h2>
            <div style="background: #e8f4f8; padding: 15px; border-radius: 5px; margin-bottom: 20px; border-left: 4px solid #007bff;">
                <h4 style="margin: 0 0 10px 0; color: #007bff;">Note Content Examples:</h4>
                <p style="margin: 5px 0; font-size: 14px;"><strong>Simple text:</strong> Drops from raid bosses</p>
                <p style="margin: 5px 0; font-size: 14px;"><strong>With link:</strong> See &lt;a href="https://wiki.example.com/quest"&gt;quest guide&lt;/a&gt; for details</p>
                <p style="margin: 5px 0; font-size: 14px;"><strong>With formatting:</strong> &lt;strong&gt;Important:&lt;/strong&gt; Level 65+ required</p>
            </div>
            {}
            {}
        </div>
    </div>
</body>
</html>
        "#,
        note_types_html,
        if note_types.is_empty() {
            "<p>No note types yet.</p>"
        } else {
            ""
        }
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
async fn create_note_type(
    State(state): State<AppState>,
    Form(form): Form<NoteTypeForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    let _ =
        sqlx::query("INSERT INTO note_types (name, display_name, color_class) VALUES (?, ?, ?)")
            .bind(&form.name)
            .bind(&form.display_name)
            .bind(&form.color_class)
            .execute(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to("/admin/note-types"))
}

#[cfg(feature = "admin")]
async fn delete_note_type(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // First check if the note type is being used
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM zone_notes WHERE note_type_id = ?")
        .bind(id)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if count > 0 {
        // If note type is in use, we might want to handle this differently
        // For now, we'll prevent deletion
        return Err(StatusCode::BAD_REQUEST);
    }

    let _ = sqlx::query("DELETE FROM note_types WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to("/admin/note-types"))
}

#[cfg(feature = "admin")]
async fn delete_rating_admin(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<axum::response::Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    sqlx::query("DELETE FROM zone_ratings WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(axum::response::Redirect::to("/admin/ratings"))
}

#[cfg(feature = "admin")]
async fn handle_rating_delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<axum::response::Redirect, StatusCode> {
    if form.get("_method") == Some(&"DELETE".to_string()) {
        delete_rating_admin(State(state), Path(id)).await
    } else {
        Err(StatusCode::METHOD_NOT_ALLOWED)
    }
}

#[cfg(feature = "admin")]
async fn list_links(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    let links = sqlx::query(
        "SELECT id, name, url, category, description, created_at
         FROM links
         ORDER BY category, name COLLATE NOCASE",
    )
    .fetch_all(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut html = String::from(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Manage Links - EQ RNG Admin</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }
        .nav { background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }
        .nav a { margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }
        .nav a:hover { color: #007bff; }
        table { width: 100%; border-collapse: collapse; margin-bottom: 20px; }
        th, td { padding: 10px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background-color: #f5f5f5; }
        .btn { display: inline-block; padding: 6px 12px; margin: 2px; text-decoration: none; border-radius: 4px; font-size: 12px; }
        .btn-primary { background-color: #007bff; color: white; }
        .btn-danger { background-color: #dc3545; color: white; }
        .btn-success { background-color: #28a745; color: white; }
        .category-header { background-color: #e9ecef; font-weight: bold; }
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/note-types">Manage Note Types</a>
        <a href="/admin/ratings">Manage Ratings</a>
        <a href="/admin/links">Manage Links</a>
    </div>

    <h1>Manage Links</h1>
    <p><a href="/admin/links/new" class="btn btn-success">Add New Link</a></p>

    <table>
        <thead>
            <tr>
                <th>Category</th>
                <th>Name</th>
                <th>URL</th>
                <th>Description</th>
                <th>Created</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
    "#,
    );

    let mut current_category = String::new();
    for row in links {
        let id: i32 = row.get("id");
        let name: String = row.get("name");
        let url: String = row.get("url");
        let category: String = row.get("category");
        let description: Option<String> = row.get("description");
        let created_at: String = row.get("created_at");

        if category != current_category {
            current_category = category.clone();
        }

        html.push_str(&format!(
            r#"
            <tr>
                <td>{}</td>
                <td>{}</td>
                <td><a href="{}" target="_blank">{}</a></td>
                <td>{}</td>
                <td>{}</td>
                <td>
                    <a href="/admin/links/{}" class="btn btn-primary">Edit</a>
                    <form style="display: inline;" method="post" action="/admin/links/{}/delete" onsubmit="return confirm('Are you sure you want to delete this link?');">
                        <button type="submit" class="btn btn-danger">Delete</button>
                    </form>
                </td>
            </tr>
            "#,
            category,
            name,
            url,
            if url.len() > 50 { &url[..50] } else { &url },
            description.unwrap_or("".to_string()),
            created_at.split('T').next().unwrap_or(&created_at),
            id,
            id
        ));
    }

    html.push_str(
        r#"
        </tbody>
    </table>
</body>
</html>
    "#,
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
async fn new_link_form() -> Result<Html<String>, StatusCode> {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Add New Link - EQ RNG Admin</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        .nav { background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }
        .nav a { margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }
        .nav a:hover { color: #007bff; }
        .form-group { margin-bottom: 15px; }
        label { display: block; margin-bottom: 5px; font-weight: bold; }
        input, select, textarea { width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }
        textarea { height: 80px; resize: vertical; }
        .btn { display: inline-block; padding: 10px 20px; background-color: #007bff; color: white; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }
        .btn:hover { background-color: #0056b3; }
        .btn-secondary { background-color: #6c757d; }
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/note-types">Manage Note Types</a>
        <a href="/admin/ratings">Manage Ratings</a>
        <a href="/admin/links">Manage Links</a>
    </div>

    <h1>Add New Link</h1>

    <form method="post" action="/admin/links">
        <div class="form-group">
            <label for="name">Name *</label>
            <input type="text" id="name" name="name" required>
        </div>

        <div class="form-group">
            <label for="url">URL *</label>
            <input type="url" id="url" name="url" required>
        </div>

        <div class="form-group">
            <label for="category">Category *</label>
            <select id="category" name="category" required>
                <option value="">Select a category</option>
                <option value="General">General</option>
                <option value="Class Discords">Class Discords</option>
                <option value="Content Creators">Content Creators</option>
            </select>
        </div>

        <div class="form-group">
            <label for="description">Description</label>
            <textarea id="description" name="description" placeholder="Optional description"></textarea>
        </div>

        <button type="submit" class="btn">Create Link</button>
        <a href="/admin/links" class="btn btn-secondary">Cancel</a>
    </form>
</body>
</html>
    "#;

    Ok(Html(html.to_string()))
}

#[cfg(feature = "admin")]
async fn edit_link_form(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    let link = sqlx::query("SELECT * FROM links WHERE id = ?")
        .bind(id)
        .fetch_optional(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let name: String = link.get("name");
    let url: String = link.get("url");
    let category: String = link.get("category");
    let description: Option<String> = link.get("description");

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
        .form-group {{ margin-bottom: 15px; }}
        label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        input, select, textarea {{ width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }}
        textarea {{ height: 80px; resize: vertical; }}
        .btn {{ display: inline-block; padding: 10px 20px; background-color: #007bff; color: white; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }}
        .btn:hover {{ background-color: #0056b3; }}
        .btn-secondary {{ background-color: #6c757d; }}
        .btn-danger {{ background-color: #dc3545; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/note-types">Manage Note Types</a>
        <a href="/admin/ratings">Manage Ratings</a>
        <a href="/admin/links">Manage Links</a>
    </div>

    <h1>Edit Link</h1>

    <form method="post" action="/admin/links/{}">
        <div class="form-group">
            <label for="name">Name *</label>
            <input type="text" id="name" name="name" value="{}" required>
        </div>

        <div class="form-group">
            <label for="url">URL *</label>
            <input type="url" id="url" name="url" value="{}" required>
        </div>

        <div class="form-group">
            <label for="category">Category *</label>
            <select id="category" name="category" required>
                <option value="General"{}>General</option>
                <option value="Class Discords"{}>Class Discords</option>
                <option value="Content Creators"{}>Content Creators</option>
            </select>
        </div>

        <div class="form-group">
            <label for="description">Description</label>
            <textarea id="description" name="description" placeholder="Optional description">{}</textarea>
        </div>

        <input type="hidden" name="_method" value="PUT">
        <button type="submit" class="btn">Update Link</button>
        <a href="/admin/links" class="btn btn-secondary">Cancel</a>
    </form>

    <form style="margin-top: 20px;" method="post" action="/admin/links/{}/delete" onsubmit="return confirm('Are you sure you want to delete this link?');">
        <button type="submit" class="btn btn-danger">Delete Link</button>
    </form>
</body>
</html>
    "#,
        id,
        name,
        url,
        if category == "General" {
            " selected"
        } else {
            ""
        },
        if category == "Class Discords" {
            " selected"
        } else {
            ""
        },
        if category == "Content Creators" {
            " selected"
        } else {
            ""
        },
        description.unwrap_or("".to_string()),
        id
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
async fn create_link_admin(
    State(state): State<AppState>,
    Form(form): Form<LinkForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    let _ = sqlx::query("INSERT INTO links (name, url, category, description) VALUES (?, ?, ?, ?)")
        .bind(&form.name)
        .bind(&form.url)
        .bind(&form.category)
        .bind(&form.description)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to("/admin/links"))
}

#[cfg(feature = "admin")]
async fn handle_link_update_or_delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<LinkForm>,
) -> Result<axum::response::Redirect, StatusCode> {
    if form._method.as_deref() == Some("PUT") {
        update_link_admin(State(state), Path(id), Form(form)).await
    } else {
        Err(StatusCode::METHOD_NOT_ALLOWED)
    }
}

#[cfg(feature = "admin")]
async fn update_link_admin(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<LinkForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    let _ = sqlx::query(
        "UPDATE links SET name = ?, url = ?, category = ?, description = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&form.name)
    .bind(&form.url)
    .bind(&form.category)
    .bind(&form.description)
    .bind(id)
    .execute(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Force WAL checkpoint to immediately update main database file
    let _ = crate::checkpoint_wal(pool.as_ref()).await;

    Ok(Redirect::to("/admin/links"))
}
