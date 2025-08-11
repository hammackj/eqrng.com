// Zone management functionality
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
use crate::admin::dashboard::{
    generate_expansion_options, generate_sortable_header, generate_zone_type_options,
    get_distinct_expansions, get_distinct_zone_types,
};
#[cfg(feature = "admin")]
use crate::admin::types::*;

#[cfg(feature = "admin")]
pub async fn list_zones(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Html<String>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(5, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.clone().unwrap_or_default();
    let sort = params.sort.clone().unwrap_or_else(|| "name".to_string());
    let order = params.order.clone().unwrap_or_else(|| "asc".to_string());
    let verified = params.verified.clone();
    let zone_type = params.zone_type.clone();
    let expansion = params.expansion.clone();

    let flags = params.flags.clone();

    let pool = &state.zone_state.pool;

    // Get distinct expansions for filter dropdown
    let expansions = get_distinct_expansions(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Validate sort column and order
    let valid_columns = [
        "id",
        "name",
        "level_ranges",
        "expansion",
        "zone_type",
        "rating",
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

    if params.zone_type.is_some() {
        where_conditions.push("LOWER(zone_type) LIKE LOWER(?)".to_string());
    }

    if params.expansion.is_some() {
        where_conditions.push("LOWER(expansion) LIKE LOWER(?)".to_string());
    }

    if params.flags.is_some() {
        where_conditions.push("EXISTS (SELECT 1 FROM zone_flags zf JOIN flag_types ft ON zf.flag_type_id = ft.id WHERE zf.zone_id = zones.id AND LOWER(ft.name) = LOWER(?))".to_string());
    }

    let where_clause = if where_conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", where_conditions.join(" AND "))
    };

    // Get total count
    let count_query = format!("SELECT COUNT(*) as count FROM zones {}", where_clause);
    let mut count_query_builder = sqlx::query(&count_query);

    if let Some(ref search_term) = search_param {
        count_query_builder = count_query_builder
            .bind(search_term)
            .bind(search_term)
            .bind(search_term);
    }

    if let Some(ref zone_type) = params.zone_type {
        count_query_builder = count_query_builder.bind(format!("%{}%", zone_type));
    }

    if let Some(ref expansion) = params.expansion {
        count_query_builder = count_query_builder.bind(format!("%{}%", expansion));
    }

    if let Some(ref flag_name) = params.flags {
        count_query_builder = count_query_builder.bind(flag_name);
    }

    let total_count: i32 = count_query_builder
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get("count");

    // Get zones
    let zones_query = format!(
        "SELECT id, name, level_ranges, expansion, continent, zone_type, connections, image_url, map_url, rating, verified FROM zones {} ORDER BY {} {} LIMIT ? OFFSET ?",
        where_clause, sort_column, sort_order
    );

    let mut zones_query_builder = sqlx::query(&zones_query);

    if let Some(ref search_term) = search_param {
        zones_query_builder = zones_query_builder
            .bind(search_term)
            .bind(search_term)
            .bind(search_term);
    }

    if let Some(ref zone_type) = params.zone_type {
        zones_query_builder = zones_query_builder.bind(format!("%{}%", zone_type));
    }

    if let Some(ref expansion) = params.expansion {
        zones_query_builder = zones_query_builder.bind(format!("%{}%", expansion));
    }

    if let Some(ref flag_name) = params.flags {
        zones_query_builder = zones_query_builder.bind(flag_name);
    }

    let zone_rows = zones_query_builder
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Load notes and flags for each zone
    let mut zones = Vec::new();
    for row in zone_rows {
        let zone_id: i64 = row.get("id");
        let notes = crate::zones::get_zone_notes(pool.as_ref(), zone_id)
            .await
            .unwrap_or_default();
        let flags = crate::zones::get_zone_flags(pool.as_ref(), zone_id)
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
            verified: row.get("verified"),
            notes,
            flags,
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
        th {{ background-color: #f8f9fa; border-bottom: 2px solid #dee2e6; }}
        th a:hover {{ background-color: #e9ecef; padding: 4px; border-radius: 3px; }}
        .flag-pills {{ display: flex; flex-wrap: wrap; gap: 4px; max-width: 150px; }}
        .flag-pill {{ display: inline-block; padding: 4px 8px; margin: 2px; border-radius: 12px; color: white; font-size: 0.75em; font-weight: bold; white-space: nowrap; }}
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
        <a href="/admin/instances">Manage Instances</a>
        <a href="/admin/zones/new">Add New Zone</a>
    </div>

    <h1>Manage Zones{}</h1>

    <div class="controls">
        <form method="get" style="display: flex; gap: 10px; align-items: center; flex-wrap: wrap;">
            <input type="text" name="search" placeholder="Search zones..." value="{}" />
            <select name="expansion" onchange="this.form.submit()">
                <option value="">All Expansions</option>
                {}
            </select>
            <input type="hidden" name="page" value="1" />
            <input type="hidden" name="per_page" value="{}" />
            <button type="submit" class="btn">Search</button>
        </form>
        <a href="/admin/zones/new" class="btn">Add New Zone</a>
        {}
    </div>

    <p>Showing {} zones (Page {} of {})</p>

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
                <th>Notes</th>
                <th>Flags</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
"#,
        // Filter indicator for title
        if verified.is_some() || zone_type.is_some() || expansion.is_some() || flags.is_some() {
            let mut filters = Vec::new();
            if let Some(ref v) = verified {
                filters.push(if v == "true" {
                    "Verified".to_string()
                } else {
                    "Unverified".to_string()
                });
            }
            if let Some(ref zt) = zone_type {
                filters.push(format!("{} Zones", zt.to_uppercase()));
            }
            if let Some(ref exp) = expansion {
                filters.push(format!("{} Expansion", exp));
            }
            if let Some(ref f) = flags {
                filters.push(format!("{} Flag", f.replace("_", " ").to_uppercase()));
            }
            format!(" - {}", filters.join(", "))
        } else {
            String::new()
        },
        search,
        generate_expansion_options(&expansions, &expansion.clone().unwrap_or_default()),
        per_page,
        // Clear filters link
        if verified.is_some() || zone_type.is_some() || expansion.is_some() || flags.is_some() {
            r#"<a href="/admin/zones" class="btn" style="background: #6c757d;">Clear Filters</a>"#
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
            "/admin/zones",
            &search,
        ),
        generate_sortable_header(
            "name",
            "Name",
            &params.sort,
            &params.order,
            "/admin/zones",
            &search,
        ),
        generate_sortable_header(
            "expansion",
            "Expansion",
            &params.sort,
            &params.order,
            "/admin/zones",
            &search,
        ),
        generate_sortable_header(
            "zone_type",
            "Zone Type",
            &params.sort,
            &params.order,
            "/admin/zones",
            &search,
        ),
        generate_sortable_header(
            "level_ranges",
            "Level Ranges",
            &params.sort,
            &params.order,
            "/admin/zones",
            &search,
        ),
        generate_sortable_header(
            "rating",
            "Rating",
            &params.sort,
            &params.order,
            "/admin/zones",
            &search,
        ),
        generate_sortable_header(
            "verified",
            "Verified",
            &params.sort,
            &params.order,
            "/admin/zones",
            &search,
        ),
    );

    for zone in zones {
        let (flags_title, flags_display) = if zone.flags.is_empty() {
            (String::from("-"), String::from("-"))
        } else {
            let flag_names: Vec<String> = zone
                .flags
                .iter()
                .map(|f| {
                    f.flag_type
                        .as_ref()
                        .map(|ft| ft.display_name.as_str())
                        .unwrap_or("Unknown")
                        .to_string()
                })
                .collect();

            let pills: String = zone
                .flags
                .iter()
                .map(|f| {
                    let flag_type = f.flag_type.as_ref();
                    let display_name = flag_type
                        .map(|ft| ft.display_name.as_str())
                        .unwrap_or("Unknown");
                    let color = flag_type
                        .map(|ft| match ft.color_class.as_str() {
                            "bg-red-500" => "#ef4444",
                            "bg-orange-500" => "#f97316",
                            "bg-yellow-500" => "#eab308",
                            "bg-green-500" => "#22c55e",
                            "bg-blue-500" => "#3b82f6",
                            "bg-indigo-500" => "#6366f1",
                            "bg-purple-500" => "#a855f7",
                            "bg-pink-500" => "#ec4899",
                            "bg-gray-500" => "#6b7280",
                            "bg-slate-500" => "#64748b",
                            "bg-cyan-500" => "#06b6d4",
                            "bg-teal-500" => "#14b8a6",
                            "bg-emerald-500" => "#10b981",
                            "bg-lime-500" => "#84cc16",
                            "bg-amber-500" => "#f59e0b",
                            "bg-rose-500" => "#f43f5e",
                            "bg-fuchsia-500" => "#d946ef",
                            "bg-violet-500" => "#8b5cf6",
                            "bg-sky-500" => "#0ea5e9",
                            "bg-zinc-500" => "#71717a",
                            "bg-neutral-500" => "#737373",
                            "bg-stone-500" => "#78716c",
                            _ => "#3b82f6",
                        })
                        .unwrap_or("#3b82f6");
                    format!(
                        r#"<span class="flag-pill" style="background-color: {};">{}</span>"#,
                        color, display_name
                    )
                })
                .collect::<Vec<_>>()
                .join("");

            (
                flag_names.join(", "),
                format!(r#"<div class="flag-pills">{}</div>"#, pills),
            )
        };

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
                <td class="truncate" title="{}">{}</td>
                <td>
                    <a href="/admin/zones/{}" class="btn btn-small">Edit</a>
                    <a href="/admin/zones/{}/ratings" class="btn btn-small">Ratings</a>
                    <a href="/admin/zones/{}/notes" class="btn btn-small">Notes</a>
                    <form method="post" action="/admin/zones/{}/move-to-instances" style="display: inline;">
                        <button type="submit" class="btn btn-small" style="background: #28a745;" onclick="return confirm('Move this zone to instances?')">Move to Instances</button>
                    </form>
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
            if zone.verified { "✓" } else { "✗" },
            zone.notes.len(),
            flags_title, flags_display,
            zone.id.unwrap_or(0),
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

        let mut params = Vec::new();

        if !search.is_empty() {
            params.push(format!("search={}", urlencoding::encode(&search)));
        }

        if let Some(ref v) = verified {
            params.push(format!("verified={}", urlencoding::encode(v)));
        }

        if let Some(ref zt) = zone_type {
            params.push(format!("zone_type={}", urlencoding::encode(zt)));
        }

        if let Some(ref exp) = expansion {
            params.push(format!("expansion={}", urlencoding::encode(exp)));
        }

        if let Some(ref f) = flags {
            params.push(format!("flags={}", urlencoding::encode(f)));
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
pub async fn new_zone_form(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get distinct zone types from database
    let zone_types = get_distinct_zone_types(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let html = format!(
        "{}{}",
        get_zone_form_header(),
        get_zone_form_body(
            "Create New Zone",
            &Zone {
                id: None,
                name: String::new(),
                level_ranges: "[[1,50]]".to_string(),
                expansion: String::new(),
                continent: String::new(),
                zone_type: String::new(),
                connections: "[]".to_string(),
                image_url: String::new(),
                map_url: String::new(),
                rating: 0,
                verified: false,
                notes: Vec::new(),
                flags: Vec::new(),
            },
            "/admin/zones",
            "POST",
            "Create Zone",
            None,
            &zone_types
        )
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn edit_zone_form(
    State(state): State<AppState>,
    Path(zone_id): Path<i32>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get the zone
    let zone_row = sqlx::query("SELECT * FROM zones WHERE id = ?")
        .bind(zone_id)
        .fetch_optional(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get zone types for dropdown
    let zone_types = get_distinct_zone_types(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get notes and flags for this zone
    let notes = crate::zones::get_zone_notes(pool.as_ref(), zone_id as i64)
        .await
        .unwrap_or_default();
    let flags = crate::zones::get_zone_flags(pool.as_ref(), zone_id as i64)
        .await
        .unwrap_or_default();

    // Get available note types and flag types
    let note_types = sqlx::query("SELECT * FROM note_types ORDER BY display_name")
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default();
    let flag_types = crate::zones::get_all_flag_types(pool.as_ref())
        .await
        .unwrap_or_default();

    // Get ratings for this zone
    let ratings = sqlx::query(
        "SELECT id, rating, created_at FROM zone_ratings WHERE zone_id = ? ORDER BY created_at DESC LIMIT 10"
    )
    .bind(zone_id)
    .fetch_all(pool.as_ref())
    .await
    .unwrap_or_default();

    let zone = Zone {
        id: Some(zone_id),
        name: zone_row.get("name"),
        level_ranges: zone_row.get("level_ranges"),
        expansion: zone_row.get("expansion"),
        continent: zone_row.get("continent"),
        zone_type: zone_row.get("zone_type"),
        connections: zone_row.get("connections"),
        image_url: zone_row.get("image_url"),
        map_url: zone_row.get("map_url"),
        rating: zone_row.get("rating"),
        verified: zone_row.get("verified"),
        notes,
        flags,
    };

    let html = format!(
        "{}{}",
        get_zone_form_header(),
        get_enhanced_zone_form_body(&zone, &zone_types, &note_types, &flag_types, &ratings)
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn create_zone(
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
            connections, image_url, map_url, rating, verified
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
pub async fn delete_zone(
    State(state): State<AppState>,
    Path(zone_id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let pool = &state.zone_state.pool;

    // Delete the zone (cascade will handle related records)
    let result = sqlx::query("DELETE FROM zones WHERE id = ?")
        .bind(zone_id)
        .execute(pool.as_ref())
        .await;

    match result {
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "admin")]
pub async fn move_zone_to_instances(
    State(state): State<AppState>,
    Path(zone_id): Path<i32>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get the zone data
    let _zone_row = sqlx::query("SELECT * FROM zones WHERE id = ?")
        .bind(zone_id)
        .fetch_optional(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Insert into instances table
    let result = sqlx::query(
        r#"
        INSERT INTO instances (name, level_ranges, expansion, continent, zone_type,
                             connections, image_url, map_url, rating, hot_zone, verified)
        SELECT name, level_ranges, expansion, continent, zone_type,
               connections, image_url, map_url, rating, 0, verified
        FROM zones WHERE id = ?
        "#,
    )
    .bind(zone_id)
    .execute(pool.as_ref())
    .await;

    if result.is_ok() {
        // Delete the zone after successful copy
        let _ = sqlx::query("DELETE FROM zones WHERE id = ?")
            .bind(zone_id)
            .execute(pool.as_ref())
            .await;
    }

    Ok(Redirect::to("/admin/zones"))
}

#[cfg(feature = "admin")]
pub async fn handle_zone_update_or_delete(
    State(state): State<AppState>,
    Path(zone_id): Path<i32>,
    Form(form): Form<ZoneForm>,
) -> Result<Html<String>, StatusCode> {
    if let Some(ref method) = form._method {
        if method == "DELETE" {
            return match delete_zone(State(state), Path(zone_id)).await {
                Ok(_) => Ok(Html(format!(
                    r#"<h1>Zone Deleted</h1>
                    <p>Zone has been successfully deleted.</p>
                    <a href="/admin/zones">Return to Zones List</a>"#
                ))),
                Err(_) => Ok(Html(format!(
                    r#"<h1>Error</h1>
                    <p>Failed to delete zone.</p>
                    <a href="/admin/zones/{}">Go back</a>"#,
                    zone_id
                ))),
            };
        }
    }

    // Handle update
    update_zone(State(state), Path(zone_id), Form(form)).await
}

#[cfg(feature = "admin")]
async fn update_zone(
    State(state): State<AppState>,
    Path(zone_id): Path<i32>,
    Form(form): Form<ZoneForm>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Validate JSON fields
    if serde_json::from_str::<serde_json::Value>(&form.level_ranges).is_err() {
        return Ok(Html(format!(
            r#"<h1>Error</h1><p>Invalid level_ranges JSON format</p><a href="/admin/zones/{}">Go back</a>"#,
            zone_id
        )));
    }

    if serde_json::from_str::<serde_json::Value>(&form.connections).is_err() {
        return Ok(Html(format!(
            r#"<h1>Error</h1><p>Invalid connections JSON format</p><a href="/admin/zones/{}">Go back</a>"#,
            zone_id
        )));
    }

    let verified = form.verified.is_some();

    // Update the zone
    let result = sqlx::query(
        r#"
        UPDATE zones
        SET name = ?, level_ranges = ?, expansion = ?, continent = ?, zone_type = ?,
            connections = ?, image_url = ?, map_url = ?, rating = ?, verified = ?
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
    .bind(verified)
    .bind(zone_id)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => Ok(Html(format!(
            r#"<h1>Zone Updated</h1>
            <p>Zone "{}" has been successfully updated.</p>
            <a href="/admin/zones">Return to Zones List</a>
            <a href="/admin/zones/{}" style="margin-left: 15px;">Edit Again</a>"#,
            form.name, zone_id
        ))),
        Err(_) => Ok(Html(format!(
            r#"<h1>Error</h1>
            <p>Failed to update zone.</p>
            <a href="/admin/zones/{}">Go back</a>"#,
            zone_id
        ))),
    }
}

#[cfg(feature = "admin")]
pub async fn zone_ratings(
    State(state): State<AppState>,
    Path(zone_id): Path<i32>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get zone name
    let zone_row = sqlx::query("SELECT name FROM zones WHERE id = ?")
        .bind(zone_id)
        .fetch_optional(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let zone_name: String = zone_row.get("name");

    // Get ratings for this zone
    let rating_rows = sqlx::query(
        r#"
        SELECT id, rating, created_at, updated_at
        FROM zone_ratings
        WHERE zone_id = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(zone_id)
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
        table {{ width: 100%; border-collapse: collapse; margin-bottom: 20px; }}
        th, td {{ padding: 8px; border: 1px solid #ddd; text-align: left; }}
        th {{ background: #f8f9fa; border-bottom: 2px solid #dee2e6; }}
        .rating-stars {{ color: #ffc107; }}
        .btn {{ background: #007bff; color: white; padding: 8px 15px; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }}
        .btn-danger {{ background: #dc3545; }}
        .btn-danger:hover {{ background: #c82333; }}
        .btn-small {{ padding: 4px 8px; font-size: 0.8em; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/zones/{}">Edit Zone</a>
    </div>

    <h1>Ratings for "{}"</h1>

    <p>Total ratings: {}</p>

    <table>
        <thead>
            <tr>
                <th>ID</th>
                <th>Rating</th>
                <th>Created</th>
                <th>Updated</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
"#,
        zone_id,
        zone_name,
        rating_rows.len()
    );

    for row in rating_rows {
        let rating_id: i64 = row.get("id");
        let rating: i32 = row.get("rating");
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");

        let rating_stars = "★".repeat(rating as usize) + &"☆".repeat((5 - rating) as usize);

        html.push_str(&format!(
            r#"
            <tr>
                <td>{}</td>
                <td><span class="rating-stars">{}</span> ({})</td>
                <td>{}</td>
                <td>{}</td>
                <td>
                    <form method="post" action="/admin/ratings/{}/delete" style="display: inline;">
                        <button type="submit" class="btn btn-danger btn-small" onclick="return confirm('Delete this rating?')">Delete</button>
                    </form>
                </td>
            </tr>
            "#,
            rating_id,
            rating_stars,
            rating,
            created_at.split('T').next().unwrap_or(&created_at),
            updated_at.split('T').next().unwrap_or(&updated_at),
            rating_id
        ));
    }

    html.push_str(
        r#"
        </tbody>
    </table>

    <a href="/admin/zones" class="btn">Back to Zones</a>
</body>
</html>
"#,
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn zone_notes(
    Path(zone_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get zone name
    let zone_row = sqlx::query("SELECT name FROM zones WHERE id = ?")
        .bind(zone_id)
        .fetch_optional(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let zone_name: String = zone_row.get("name");

    // Get notes for this zone
    let notes = crate::zones::get_zone_notes(pool.as_ref(), zone_id)
        .await
        .unwrap_or_default();

    // Get available note types
    let note_types = sqlx::query("SELECT * FROM note_types ORDER BY display_name")
        .fetch_all(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Zone Notes - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .form-section {{ background: #f8f9fa; padding: 20px; margin-bottom: 20px; border-radius: 5px; }}
        .form-group {{ margin-bottom: 15px; }}
        label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        input, select, textarea {{ width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }}
        textarea {{ height: 100px; resize: vertical; }}
        table {{ width: 100%; border-collapse: collapse; margin-bottom: 20px; }}
        th, td {{ padding: 8px; border: 1px solid #ddd; text-align: left; }}
        th {{ background: #f8f9fa; border-bottom: 2px solid #dee2e6; }}
        .btn {{ background: #007bff; color: white; padding: 8px 15px; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }}
        .btn:hover {{ background: #0056b3; }}
        .btn-danger {{ background: #dc3545; }}
        .btn-danger:hover {{ background: #c82333; }}
        .btn-small {{ padding: 4px 8px; font-size: 0.8em; }}
        .note-type-badge {{ display: inline-block; padding: 4px 8px; border-radius: 12px; color: white; font-size: 0.8em; font-weight: bold; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/zones/{}">Edit Zone</a>
    </div>

    <h1>Notes for "{}"</h1>

    <div class="form-section">
        <h2>Add New Note</h2>
        <form method="post" action="/admin/zones/{}/notes">
            <div class="form-group">
                <label for="note_type_id">Note Type:</label>
                <select id="note_type_id" name="note_type_id" required>
                    <option value="">Select a note type...</option>
"#,
        zone_id, zone_name, zone_id
    );

    for note_type_row in note_types {
        let nt_id: i64 = note_type_row.get("id");
        let nt_display_name: String = note_type_row.get("display_name");
        html.push_str(&format!(
            r#"                    <option value="{}">{}</option>
"#,
            nt_id, nt_display_name
        ));
    }

    html.push_str(
        r#"                </select>
            </div>

            <div class="form-group">
                <label for="content">Content:</label>
                <textarea id="content" name="content" required placeholder="Enter note content..."></textarea>
            </div>

            <div class="form-group">
                <button type="submit" class="btn">Add Note</button>
            </div>
        </form>
    </div>

    <h2>Existing Notes</h2>

    <table>
        <thead>
            <tr>
                <th>Type</th>
                <th>Content</th>
                <th>Created</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
"#,
    );

    for note in notes {
        let note_type_display = note
            .note_type
            .as_ref()
            .map(|nt| nt.display_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        let note_type_color = note
            .note_type
            .as_ref()
            .map(|nt| nt.color_class.clone())
            .unwrap_or_else(|| "bg-gray-500".to_string());

        html.push_str(&format!(
            r#"
            <tr>
                <td><span class="note-type-badge {}">{}</span></td>
                <td>{}</td>
                <td>Just now</td>
                <td>
                    <form method="post" action="/admin/zones/{}/notes/{}/delete" style="display: inline;">
                        <button type="submit" class="btn btn-danger btn-small" onclick="return confirm('Delete this note?')">Delete</button>
                    </form>
                </td>
            </tr>
            "#,
            note_type_color,
            note_type_display,
            note.content,
            zone_id,
            note.id.unwrap_or(0)
        ));
    }

    html.push_str(
        r#"
        </tbody>
    </table>

    <a href="/admin/zones" class="btn">Back to Zones</a>
</body>
</html>
"#,
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn create_zone_note(
    Path(zone_id): Path<i64>,
    State(state): State<AppState>,
    Form(form): Form<ZoneNoteForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Insert the new note
    let result = sqlx::query(
        r#"
        INSERT INTO zone_notes (zone_id, note_type_id, content)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(zone_id)
    .bind(form.note_type_id)
    .bind(&form.content)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => Ok(Redirect::to(&format!("/admin/zones/{}", zone_id))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "admin")]
pub async fn delete_zone_note(
    State(state): State<AppState>,
    Path((zone_id, note_id)): Path<(i64, i64)>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Delete the note
    let result = sqlx::query("DELETE FROM zone_notes WHERE id = ? AND zone_id = ?")
        .bind(note_id)
        .bind(zone_id)
        .execute(pool.as_ref())
        .await;

    match result {
        Ok(_) => Ok(Redirect::to(&format!("/admin/zones/{}", zone_id))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "admin")]
pub async fn create_zone_flag(
    State(state): State<AppState>,
    Path(zone_id): Path<i64>,
    Form(form): Form<ZoneFlagForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Insert the new flag (use INSERT OR IGNORE to handle duplicates)
    let result = sqlx::query(
        r#"
        INSERT OR IGNORE INTO zone_flags (zone_id, flag_type_id)
        VALUES (?, ?)
        "#,
    )
    .bind(zone_id)
    .bind(form.flag_type_id)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => Ok(Redirect::to(&format!("/admin/zones/{}", zone_id))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "admin")]
pub async fn delete_zone_flag_simple(
    State(state): State<AppState>,
    Path((zone_id, flag_id)): Path<(i64, i64)>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Delete the flag
    let result = sqlx::query("DELETE FROM zone_flags WHERE id = ? AND zone_id = ?")
        .bind(flag_id)
        .bind(zone_id)
        .execute(pool.as_ref())
        .await;

    match result {
        Ok(_) => Ok(Redirect::to(&format!("/admin/zones/{}", zone_id))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "admin")]
pub async fn delete_zone_flag(
    State(state): State<AppState>,
    Path((zone_id, flag_id)): Path<(i64, i64)>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Delete the flag
    let result = sqlx::query("DELETE FROM zone_flags WHERE id = ? AND zone_id = ?")
        .bind(flag_id)
        .bind(zone_id)
        .execute(pool.as_ref())
        .await;

    match result {
        Ok(_) => Ok(Redirect::to(&format!("/admin/zones/{}", zone_id))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(feature = "admin")]
fn get_zone_form_header() -> String {
    r#"
<!DOCTYPE html>
<html>
<head>
    <title>Zone Form - EQ RNG Admin</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 1400px; margin: 0 auto; padding: 20px; }
        .nav { background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }
        .nav a { margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }
        .nav a:hover { color: #007bff; }
        .form-group { margin-bottom: 15px; }
        label { display: block; margin-bottom: 5px; font-weight: bold; }
        input, textarea, select { width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }
        textarea { height: 80px; resize: vertical; }
        .checkbox-group { display: flex; align-items: center; gap: 10px; }
        .checkbox-group input { width: auto; }
        .btn { background: #007bff; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; text-decoration: none; display: inline-block; }
        .btn:hover { background: #0056b3; }
        .btn-secondary { background: #6c757d; margin-left: 10px; }
        .btn-secondary:hover { background: #545b62; }
        .btn-warning { background: #ffc107; color: #212529; }
        .btn-warning:hover { background: #e0a800; }
        .btn-small { padding: 5px 10px; font-size: 12px; }

        /* Management sections styles */
        .management-section {
            background: #f8f9fa;
            border: 1px solid #dee2e6;
            border-radius: 5px;
            padding: 15px;
            margin-bottom: 20px;
        }
        .management-section h3 {
            margin: 0 0 15px 0;
            color: #495057;
            border-bottom: 2px solid #dee2e6;
            padding-bottom: 5px;
        }
        .current-items {
            margin-bottom: 15px;
            min-height: 40px;
        }
        .no-items {
            color: #6c757d;
            font-style: italic;
            margin: 10px 0;
        }

        /* Flag and note items */
        .item-pill {
            display: inline-flex;
            align-items: center;
            background: white;
            border: 1px solid #dee2e6;
            border-radius: 15px;
            padding: 5px 10px;
            margin: 3px;
            font-size: 12px;
        }
        .flag-badge, .note-badge {
            padding: 2px 6px;
            border-radius: 3px;
            color: white;
            font-weight: bold;
            margin-right: 5px;
        }
        .bg-gray-500 { background-color: #6c757d; }
        .bg-blue-500 { background-color: #3b82f6; }
        .bg-green-500 { background-color: #22c55e; }
        .bg-yellow-500 { background-color: #eab308; color: #212529; }
        .bg-red-500 { background-color: #ef4444; }
        .bg-purple-500 { background-color: #a855f7; }
        .bg-indigo-500 { background-color: #6366f1; }
        .bg-pink-500 { background-color: #ec4899; }
        .bg-orange-500 { background-color: #f97316; }
        .bg-teal-500 { background-color: #14b8a6; }
        .bg-cyan-500 { background-color: #06b6d4; }
        .bg-emerald-500 { background-color: #10b981; }
        .bg-lime-500 { background-color: #84cc16; color: #212529; }
        .bg-amber-500 { background-color: #f59e0b; color: #212529; }
        .bg-rose-500 { background-color: #f43f5e; }
        .bg-fuchsia-500 { background-color: #d946ef; }
        .bg-violet-500 { background-color: #8b5cf6; }
        .bg-slate-500 { background-color: #64748b; }
        .bg-zinc-500 { background-color: #71717a; }
        .bg-neutral-500 { background-color: #737373; }
        .bg-stone-500 { background-color: #78716c; }
        .bg-sky-500 { background-color: #0ea5e9; }

        .remove-link {
            color: #dc3545;
            text-decoration: none;
            font-weight: bold;
            margin-left: 5px;
            cursor: pointer;
        }
        .remove-link:hover { color: #c82333; }

        /* Note items */
        .note-item {
            background: white;
            border: 1px solid #dee2e6;
            border-radius: 5px;
            padding: 10px;
            margin-bottom: 10px;
        }
        .note-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 5px;
        }
        .note-content {
            font-size: 14px;
            line-height: 1.4;
            color: #495057;
        }

        /* Add forms */
        .add-form {
            display: flex;
            gap: 10px;
            align-items: end;
        }
        .add-form select {
            flex: 1;
            max-width: 200px;
        }
        .add-form textarea {
            flex: 2;
            height: 40px;
            resize: vertical;
        }

        /* Ratings */
        .ratings-list {
            margin-bottom: 15px;
        }
        .rating-item {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 5px 0;
            border-bottom: 1px solid #dee2e6;
        }
        .rating-item:last-child {
            border-bottom: none;
        }
        .rating-stars {
            color: #ffc107;
            font-size: 16px;
        }
        .rating-stars small {
            color: #6c757d;
            font-size: 12px;
        }
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
fn get_enhanced_zone_form_body(
    zone: &Zone,
    zone_types: &[String],
    note_types: &[sqlx::sqlite::SqliteRow],
    flag_types: &[crate::zones::FlagType],
    ratings: &[sqlx::sqlite::SqliteRow],
) -> String {
    use sqlx::Row;

    let zone_id = zone.id.unwrap_or(0);

    format!(
        r#"
    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 30px;">
        <!-- Left Column: Zone Form -->
        <div>
            <h2>Edit Zone</h2>
            <form method="post" action="/admin/zones/{}">
                <input type="hidden" name="_method" value="PUT" />

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
                    <label for="zone_type">Zone Type:</label>
                    <select id="zone_type" name="zone_type" required>
                        {}
                    </select>
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
                    <input type="checkbox" id="verified" name="verified" value="true" {}>
                    <label for="verified">Verified</label>
                </div>

                <div class="form-group">
                    <button type="submit" class="btn">Update Zone</button>
                    <a href="/admin/zones" class="btn btn-secondary">Cancel</a>
                    <a href="/admin/zones/{}/move-to-instances" class="btn btn-warning"
                       onclick="return confirm('Convert this zone to an instance?')">Convert to Instance</a>
                </div>
            </form>
        </div>

        <!-- Right Column: Management Sections -->
        <div>
            <!-- Flags Section -->
            <div class="management-section">
                <h3>Zone Flags</h3>
                <div class="current-items">
                    {}
                </div>
                <form method="post" action="/admin/zones/{}/flags" class="add-form">
                    <select name="flag_type_id" required>
                        <option value="">Add flag...</option>
                        {}
                    </select>
                    <button type="submit" class="btn btn-small">Add</button>
                </form>
            </div>

            <!-- Notes Section -->
            <div class="management-section">
                <h3>Zone Notes</h3>
                <div class="current-items">
                    {}
                </div>
                <form method="post" action="/admin/zones/{}/notes" class="add-form">
                    <select name="note_type_id" required>
                        <option value="">Select note type...</option>
                        {}
                    </select>
                    <textarea name="content" placeholder="Note content..." required></textarea>
                    <button type="submit" class="btn btn-small">Add Note</button>
                </form>
            </div>

            <!-- Ratings Section -->
            <div class="management-section">
                <h3>Recent Ratings</h3>
                <div class="ratings-list">
                    {}
                </div>
                <a href="/admin/zones/{}/ratings" class="btn btn-small">View All Ratings</a>
            </div>
        </div>
    </div>

</body>
</html>
"#,
        zone_id,
        zone.name.replace('"', "&quot;"),
        zone.level_ranges.replace('"', "&quot;"),
        zone.expansion.replace('"', "&quot;"),
        zone.continent.replace('"', "&quot;"),
        generate_zone_type_options(zone_types, &zone.zone_type),
        zone.connections.replace('"', "&quot;"),
        zone.image_url.replace('"', "&quot;"),
        zone.map_url.replace('"', "&quot;"),
        zone.rating,
        if zone.verified { "checked" } else { "" },
        zone_id,
        // Current flags
        if zone.flags.is_empty() {
            "<p class=\"no-items\">No flags assigned</p>".to_string()
        } else {
            zone.flags
                .iter()
                .map(|flag| {
                    let flag_id = flag.id.unwrap_or(0);
                    let flag_display = flag
                        .flag_type
                        .as_ref()
                        .map(|ft| ft.display_name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let flag_color = flag
                        .flag_type
                        .as_ref()
                        .map(|ft| ft.color_class.clone())
                        .unwrap_or_else(|| "bg-gray-500".to_string());

                    format!(
                        r#"<div class="item-pill">
                        <span class="flag-badge {}">{}</span>
                        <a href="/admin/zones/{}/remove-flag/{}" class="remove-link"
                           onclick="return confirm('Remove this flag?')">×</a>
                    </div>"#,
                        flag_color, flag_display, zone_id, flag_id
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        },
        zone_id,
        // Available flag types
        flag_types
            .iter()
            .map(|ft| {
                let ft_id = ft.id.unwrap_or(0);
                format!(r#"<option value="{}">{}</option>"#, ft_id, ft.display_name)
            })
            .collect::<Vec<_>>()
            .join(""),
        // Current notes
        if zone.notes.is_empty() {
            "<p class=\"no-items\">No notes added</p>".to_string()
        } else {
            zone.notes
                .iter()
                .map(|note| {
                    let note_id = note.id.unwrap_or(0);
                    let note_type_display = note
                        .note_type
                        .as_ref()
                        .map(|nt| nt.display_name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let note_type_color = note
                        .note_type
                        .as_ref()
                        .map(|nt| nt.color_class.clone())
                        .unwrap_or_else(|| "bg-gray-500".to_string());

                    format!(
                        r#"<div class="note-item">
                        <div class="note-header">
                            <span class="note-badge {}">{}</span>
                            <a href="/admin/zones/{}/notes/{}/delete" class="remove-link"
                               onclick="return confirm('Delete this note?')">×</a>
                        </div>
                        <div class="note-content">{}</div>
                    </div>"#,
                        note_type_color,
                        note_type_display,
                        zone_id,
                        note_id,
                        note.content.replace('<', "&lt;").replace('>', "&gt;")
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        },
        zone_id,
        // Available note types
        note_types
            .iter()
            .map(|nt| {
                let nt_id: i64 = nt.get("id");
                let nt_display: String = nt.get("display_name");
                format!(r#"<option value="{}">{}</option>"#, nt_id, nt_display)
            })
            .collect::<Vec<_>>()
            .join(""),
        // Recent ratings
        if ratings.is_empty() {
            "<p class=\"no-items\">No ratings yet</p>".to_string()
        } else {
            ratings
                .iter()
                .take(5)
                .map(|rating| {
                    let rating_val: i32 = rating.get("rating");
                    let created_at: String = rating.get("created_at");
                    let stars =
                        "★".repeat(rating_val as usize) + &"☆".repeat((5 - rating_val) as usize);

                    format!(
                        r#"<div class="rating-item">
                        <span class="rating-stars">{}</span>
                        <small>{}</small>
                    </div>"#,
                        stars,
                        created_at.split('T').next().unwrap_or(&created_at)
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        },
        zone_id
    )
}

fn get_zone_form_body(
    title: &str,
    zone: &Zone,
    action: &str,
    method: &str,
    button_text: &str,
    _zone_id: Option<i32>,
    zone_types: &[String],
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
                {}
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
        generate_zone_type_options(zone_types, &zone.zone_type),
        zone.level_ranges,
        zone.connections,
        zone.image_url,
        zone.map_url,
        zone.rating,
        if zone.verified { "checked" } else { "" },
        button_text,
    )
}
