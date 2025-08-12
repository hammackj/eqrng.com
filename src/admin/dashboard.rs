#[cfg(feature = "admin")]
use axum::{extract::State, http::StatusCode, response::Html};
#[cfg(feature = "admin")]
use sqlx::{Row, SqlitePool};
#[cfg(feature = "admin")]
use urlencoding;

use crate::AppState;

#[cfg(feature = "admin")]
pub fn generate_sortable_header(
    column: &str,
    display_name: &str,
    current_sort: &Option<String>,
    current_order: &Option<String>,
    base_url: &str,
    search: &str,
) -> String {
    let is_current = current_sort.as_ref().map_or(false, |s| s == column);
    let next_order = if is_current && current_order.as_ref().map_or("", |o| o) == "asc" {
        "desc"
    } else {
        "asc"
    };

    let search_param = if search.is_empty() {
        String::new()
    } else {
        format!("&search={}", urlencoding::encode(search))
    };

    let arrow = if is_current {
        match current_order.as_ref().map_or("", |o| o) {
            "asc" => " <span style=\"color: #007bff;\">↑</span>",
            "desc" => " <span style=\"color: #007bff;\">↓</span>",
            _ => "",
        }
    } else {
        " <span style=\"color: #ccc; font-size: 0.8em;\">↕</span>"
    };

    let link_style = if is_current {
        "text-decoration: none; color: #007bff; font-weight: bold; cursor: pointer;"
    } else {
        "text-decoration: none; color: #333; font-weight: bold; cursor: pointer;"
    };

    format!(
        r#"<a href="{}?sort={}&order={}{}" style="{}" title="Sort by {}">{}{}</a>"#,
        base_url, column, next_order, search_param, link_style, display_name, display_name, arrow
    )
}

#[cfg(feature = "admin")]
pub async fn get_distinct_zone_types(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT DISTINCT zone_type FROM zones ORDER BY zone_type ASC")
        .fetch_all(pool)
        .await?;

    let zone_types = rows
        .iter()
        .map(|row| row.get::<String, _>("zone_type"))
        .collect();

    Ok(zone_types)
}

#[cfg(feature = "admin")]
pub fn generate_zone_type_options(zone_types: &[String], selected_zone_type: &str) -> String {
    let mut options = String::new();

    for zone_type in zone_types {
        let selected = if zone_type == selected_zone_type {
            " selected"
        } else {
            ""
        };
        options.push_str(&format!(
            r#"                <option value="{}"{}>{}</option>
"#,
            zone_type, selected, zone_type
        ));
    }

    options
}

#[cfg(feature = "admin")]
pub async fn get_distinct_expansions(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT DISTINCT expansion FROM zones ORDER BY expansion ASC")
        .fetch_all(pool)
        .await?;

    let expansions = rows
        .iter()
        .map(|row| row.get::<String, _>("expansion"))
        .collect();

    Ok(expansions)
}

#[cfg(feature = "admin")]
pub fn generate_expansion_options(expansions: &[String], selected_expansion: &str) -> String {
    let mut options = String::new();

    for expansion in expansions {
        let selected = if expansion == selected_expansion {
            " selected"
        } else {
            ""
        };
        options.push_str(&format!(
            r#"                <option value="{}"{}>{}</option>
"#,
            expansion, selected, expansion
        ));
    }

    options
}

#[cfg(feature = "admin")]
pub async fn log_admin_requests(
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let _method = request.method().clone();
    let _uri = request.uri().clone();
    let _headers = request.headers().clone();

    // Log admin requests for security monitoring (without sensitive details)
    // TODO: Replace with proper structured logging

    let response = next.run(request).await;
    let _status = response.status();

    response
}

#[cfg(feature = "admin")]
pub async fn admin_dashboard(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;
    let instance_pool = &state.instance_state.pool;

    // Get statistics
    let zone_count: i32 = sqlx::query("SELECT COUNT(*) as count FROM zones")
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get("count");

    let instance_count: i32 = sqlx::query("SELECT COUNT(*) as count FROM instances")
        .fetch_one(instance_pool.as_ref())
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

    // Get zone type counts
    let outdoor_zone_count: i32 =
        sqlx::query("SELECT COUNT(*) as count FROM zones WHERE LOWER(zone_type) LIKE '%outdoor%'")
            .fetch_one(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get("count");

    let dungeon_zone_count: i32 =
        sqlx::query("SELECT COUNT(*) as count FROM zones WHERE LOWER(zone_type) LIKE '%dungeon%'")
            .fetch_one(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get("count");

    let indoor_zone_count: i32 =
        sqlx::query("SELECT COUNT(*) as count FROM zones WHERE LOWER(zone_type) LIKE '%indoor%'")
            .fetch_one(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get("count");

    let city_zone_count: i32 =
        sqlx::query("SELECT COUNT(*) as count FROM zones WHERE LOWER(zone_type) LIKE '%city%'")
            .fetch_one(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get("count");

    let raid_zone_count: i32 =
        sqlx::query("SELECT COUNT(*) as count FROM zones WHERE LOWER(zone_type) LIKE '%raid%'")
            .fetch_one(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get("count");

    // Get zone flag counts dynamically
    let flag_counts = sqlx::query(
        "SELECT ft.name, ft.display_name, ft.color_class, COUNT(DISTINCT z.id) as count FROM flag_types ft LEFT JOIN zone_flags zf ON ft.id = zf.flag_type_id LEFT JOIN zones z ON zf.zone_id = z.id GROUP BY ft.id, ft.name, ft.display_name, ft.color_class ORDER BY count DESC"
    )
    .fetch_all(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let verified_zone_count: i32 =
        sqlx::query("SELECT COUNT(*) as count FROM zones WHERE verified = 1")
            .fetch_one(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get("count");

    let unverified_zone_count: i32 =
        sqlx::query("SELECT COUNT(*) as count FROM zones WHERE verified = 0")
            .fetch_one(pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get("count");

    let verified_instance_count: i32 =
        sqlx::query("SELECT COUNT(*) as count FROM instances WHERE verified = 1")
            .fetch_one(instance_pool.as_ref())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .get("count");

    let unverified_instance_count: i32 =
        sqlx::query("SELECT COUNT(*) as count FROM instances WHERE verified = 0")
            .fetch_one(instance_pool.as_ref())
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
        .stat-card {{ background: #f8f9fa; padding: 20px; text-align: center; border-radius: 5px; text-decoration: none; color: inherit; transition: background-color 0.3s; }}
        .stat-card:hover {{ background: #e9ecef; }}
        .stat-number {{ font-size: 2em; font-weight: bold; color: #007bff; }}
        .stat-label {{ font-size: 14px; color: #666; margin-top: 5px; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/instances">Manage Instances</a>
        <a href="/admin/zones/new">Add New Zone</a>
        <a href="/admin/note-types">Manage Note Types</a>
        <a href="/admin/flag-types">Manage Flag Types</a>
        <a href="/admin/ratings">Manage Ratings</a>
        <a href="/admin/links">Manage Links</a>
    </div>

    <h1>EQ RNG Admin Dashboard</h1>

    <div class="stats">
        <a href="/admin/zones" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Total Zones</div>
        </a>
        <a href="/admin/instances" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Total Instances</div>
        </a>
        <a href="/admin/ratings" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Total Ratings</div>
        </a>
        <div class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Average Rating</div>
        </div>
        <a href="/admin/zones?zone_type=outdoor" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Outdoor Zones</div>
        </a>
        <a href="/admin/zones?zone_type=dungeon" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Dungeon Zones</div>
        </a>
        <a href="/admin/zones?zone_type=indoor" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Indoor Zones</div>
        </a>
        <a href="/admin/zones?zone_type=city" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">City Zones</div>
        </a>
        <a href="/admin/zones?zone_type=raid" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Raid Zones</div>
        </a>
    </div>

    <div class="stats">
        <div class="stat-card" style="background: #f8f9fa; border-left: 4px solid #ef4444;">
            <div class="stat-label" style="font-weight: bold; color: #495057;">Zone Flags</div>
        </div>
        {}
        <a href="/admin/zones?verified=true" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Verified Zones</div>
        </a>
        <a href="/admin/zones?verified=false" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Unverified Zones</div>
        </a>
        <a href="/admin/instances?verified=true" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Verified Instances</div>
        </a>
        <a href="/admin/instances?verified=false" class="stat-card">
            <div class="stat-number">{}</div>
            <div class="stat-label">Unverified Instances</div>
        </a>
    </div>

    <div class="card">
        <h2>Quick Actions</h2>
        <p><a href="/admin/zones">Manage all zones</a> - View, edit, delete zones, and move zones to instances</p>
        <p><a href="/admin/instances">Manage all instances</a> - View instances that were moved from zones</p>
        <p><a href="/admin/zones/new">Add new zone</a> - Create a new zone entry</p>
        <p><a href="/admin/note-types">Manage note types</a> - Configure pill icons for zone notes</p>
        <p><a href="/admin/flag-types">Manage flag types</a> - Configure zone flags and their appearance</p>

        <p><a href="/admin/ratings">Manage all ratings</a> - View and delete zone ratings</p>
        <p><a href="/admin/links">Manage links</a> - View, edit, and delete links organized by category</p>

        <div style="margin-top: 20px; padding-top: 20px; border-top: 1px solid #ddd;">
            <form action="/admin/dump-database" method="post" style="margin: 0;">
                <button type="submit" style="background-color: #dc2626; color: white; padding: 8px 16px; border: none; border-radius: 4px; cursor: pointer; font-weight: bold;"
                        onclick="return confirm('This will create a new timestamped data.sql file. Continue?')">
                    🗄️ Dump Database to SQL
                </button>
            </form>
            <p style="margin-top: 8px; font-size: 0.875rem; color: #666;">Export complete database to timestamped data-YYYYMMDD_HHMMSS.sql file</p>
        </div>
    </div>

    <div class="card">
        <h2>System Status</h2>
        <p>Admin interface is running in development mode.</p>
        <p><strong>Warning:</strong> This interface should only be used locally for development.</p>
    </div>
</body>
</html>
    "#,
        zone_count,
        instance_count,
        rating_count,
        avg_rating_display,
        outdoor_zone_count,
        dungeon_zone_count,
        indoor_zone_count,
        city_zone_count,
        raid_zone_count,
        flag_counts
            .iter()
            .map(|row| {
                let count: i32 = row.get("count");
                let name: String = row.get("name");
                let display_name: String = row.get("display_name");
                if count > 0 {
                    format!(
                        r#"<a href="/admin/zones?flags={}" class="stat-card">
                        <div class="stat-number">{}</div>
                        <div class="stat-label">{}</div>
                    </a>"#,
                        name, count, display_name
                    )
                } else {
                    String::new()
                }
            })
            .collect::<Vec<_>>()
            .join(""),
        verified_zone_count,
        unverified_zone_count,
        verified_instance_count,
        unverified_instance_count
    );
    Ok(Html(html))
}
