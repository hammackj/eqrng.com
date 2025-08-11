#[cfg(feature = "admin")]
use axum::{extract::State, http::StatusCode, response::Html};
#[cfg(feature = "admin")]
use sqlx::Row;
#[cfg(feature = "admin")]
use std::fs::{self, File};
#[cfg(feature = "admin")]
use std::io::Write;
#[cfg(feature = "admin")]
use std::path::Path;

use crate::AppState;

#[cfg(feature = "admin")]
pub async fn dump_database_sql(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;
    let instance_pool = &state.instance_state.pool;

    // Get current timestamp for filename
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y%m%d_%H%M%S");
    let filename = format!("data-{}.sql", timestamp);

    // Create data directory if it doesn't exist, then use it
    let data_dir = Path::new("data");
    if !data_dir.exists() {
        fs::create_dir_all(data_dir).map_err(|e| {
            eprintln!("Failed to create data directory: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }
    let file_path = format!("data/{}", filename);

    let mut file = File::create(&file_path).map_err(|e| {
        eprintln!("Failed to create dump file {}: {}", file_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Write header
    writeln!(
        file,
        "-- EQ RNG Database Dump\n-- Generated: {}\n-- File: {}\n",
        now.format("%Y-%m-%d %H:%M:%S UTC"),
        file_path
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut total_records = 0;
    let mut error_count = 0;

    // Helper function to escape SQL strings
    let escape_sql = |s: &str| {
        s.replace("'", "''")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
    };

    // Dump zones table
    writeln!(file, "-- Zones table").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match sqlx::query("SELECT * FROM zones ORDER BY id")
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(zones) => {
            for zone in zones {
                let id: i32 = zone.get("id");
                let name: String = zone.get("name");
                let level_ranges: String = zone.get("level_ranges");
                let expansion: String = zone.get("expansion");
                let continent: String = zone.get("continent");
                let zone_type: String = zone.get("zone_type");
                let connections: String = zone.get("connections");
                let image_url: String = zone.get("image_url");
                let map_url: String = zone.get("map_url");
                let rating: i32 = zone.get("rating");
                let verified: bool = zone.get("verified");

                if let Err(_) = writeln!(
                    file,
                    "INSERT INTO zones (id, name, level_ranges, expansion, continent, zone_type, connections, image_url, map_url, rating, verified) VALUES ({}, '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', {}, {});",
                    id,
                    escape_sql(&name),
                    escape_sql(&level_ranges),
                    escape_sql(&expansion),
                    escape_sql(&continent),
                    escape_sql(&zone_type),
                    escape_sql(&connections),
                    escape_sql(&image_url),
                    escape_sql(&map_url),
                    rating,
                    if verified { 1 } else { 0 }
                ) {
                    error_count += 1;
                }
                total_records += 1;
            }
        }
        Err(e) => {
            writeln!(file, "-- Error dumping zones: {}", e).ok();
            error_count += 1;
        }
    }

    // Dump instances table
    writeln!(file, "\n-- Instances table").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match sqlx::query("SELECT * FROM instances ORDER BY id")
        .fetch_all(instance_pool.as_ref())
        .await
    {
        Ok(instances) => {
            for instance in instances {
                let id: i32 = instance.get("id");
                let name: String = instance.get("name");
                let level_ranges: String = instance.get("level_ranges");
                let expansion: String = instance.get("expansion");
                let continent: String = instance.get("continent");
                let zone_type: String = instance.get("zone_type");
                let connections: String = instance.get("connections");
                let image_url: String = instance.get("image_url");
                let map_url: String = instance.get("map_url");
                let rating: i32 = instance.get("rating");
                let hot_zone: bool = instance.get("hot_zone");
                let verified: bool = instance.get("verified");

                if let Err(_) = writeln!(
                    file,
                    "INSERT INTO instances (id, name, level_ranges, expansion, continent, zone_type, connections, image_url, map_url, rating, hot_zone, verified) VALUES ({}, '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', {}, {}, {});",
                    id,
                    escape_sql(&name),
                    escape_sql(&level_ranges),
                    escape_sql(&expansion),
                    escape_sql(&continent),
                    escape_sql(&zone_type),
                    escape_sql(&connections),
                    escape_sql(&image_url),
                    escape_sql(&map_url),
                    rating,
                    if hot_zone { 1 } else { 0 },
                    if verified { 1 } else { 0 }
                ) {
                    error_count += 1;
                }
                total_records += 1;
            }
        }
        Err(e) => {
            writeln!(file, "-- Error dumping instances: {}", e).ok();
            error_count += 1;
        }
    }

    // Dump zone_ratings table
    writeln!(file, "\n-- Zone Ratings table").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match sqlx::query("SELECT * FROM zone_ratings ORDER BY id")
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(ratings) => {
            for rating in ratings {
                let id: i64 = rating.get("id");
                let zone_id: i64 = rating.get("zone_id");
                let user_ip: String = rating.get("user_ip");
                let rating_val: i32 = rating.get("rating");
                let created_at: String = rating.get("created_at");
                let updated_at: String = rating.get("updated_at");

                if let Err(_) = writeln!(
                    file,
                    "INSERT INTO zone_ratings (id, zone_id, user_ip, rating, created_at, updated_at) VALUES ({}, {}, '{}', {}, '{}', '{}');",
                    id,
                    zone_id,
                    escape_sql(&user_ip),
                    rating_val,
                    escape_sql(&created_at),
                    escape_sql(&updated_at)
                ) {
                    error_count += 1;
                }
                total_records += 1;
            }
        }
        Err(e) => {
            writeln!(
                file,
                "-- Error dumping zone_ratings (table may not exist): {}",
                e
            )
            .ok();
        }
    }

    // Dump note_types table
    writeln!(file, "\n-- Note Types table").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match sqlx::query("SELECT * FROM note_types ORDER BY id")
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(note_types) => {
            for note_type in note_types {
                let id: i64 = note_type.get("id");
                let name: String = note_type.get("name");
                let display_name: String = note_type.get("display_name");
                let color_class: String = note_type.get("color_class");
                let created_at: String = note_type.get("created_at");

                if let Err(_) = writeln!(
                    file,
                    "INSERT INTO note_types (id, name, display_name, color_class, created_at) VALUES ({}, '{}', '{}', '{}', '{}');",
                    id,
                    escape_sql(&name),
                    escape_sql(&display_name),
                    escape_sql(&color_class),
                    escape_sql(&created_at)
                ) {
                    error_count += 1;
                }
                total_records += 1;
            }
        }
        Err(e) => {
            writeln!(
                file,
                "-- Error dumping note_types (table may not exist): {}",
                e
            )
            .ok();
        }
    }

    // Dump flag_types table
    writeln!(file, "\n-- Flag Types table").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match sqlx::query("SELECT * FROM flag_types ORDER BY id")
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(flag_types) => {
            for flag_type in flag_types {
                let id: i64 = flag_type.get("id");
                let name: String = flag_type.get("name");
                let display_name: String = flag_type.get("display_name");
                let color_class: String = flag_type.get("color_class");
                let created_at: String = flag_type.get("created_at");

                if let Err(_) = writeln!(
                    file,
                    "INSERT INTO flag_types (id, name, display_name, color_class, created_at) VALUES ({}, '{}', '{}', '{}', '{}');",
                    id,
                    escape_sql(&name),
                    escape_sql(&display_name),
                    escape_sql(&color_class),
                    escape_sql(&created_at)
                ) {
                    error_count += 1;
                }
                total_records += 1;
            }
        }
        Err(e) => {
            writeln!(
                file,
                "-- Error dumping flag_types (table may not exist): {}",
                e
            )
            .ok();
        }
    }

    // Dump zone_notes table
    writeln!(file, "\n-- Zone Notes table").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match sqlx::query("SELECT * FROM zone_notes ORDER BY id")
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(zone_notes) => {
            for note in zone_notes {
                let id: i64 = note.get("id");
                let zone_id: i64 = note.get("zone_id");
                let note_type_id: i64 = note.get("note_type_id");
                let content: String = note.get("content");
                let created_at: String = note.get("created_at");
                let updated_at: String = note.get("updated_at");

                if let Err(_) = writeln!(
                    file,
                    "INSERT INTO zone_notes (id, zone_id, note_type_id, content, created_at, updated_at) VALUES ({}, {}, {}, '{}', '{}', '{}');",
                    id,
                    zone_id,
                    note_type_id,
                    escape_sql(&content),
                    escape_sql(&created_at),
                    escape_sql(&updated_at)
                ) {
                    error_count += 1;
                }
                total_records += 1;
            }
        }
        Err(e) => {
            writeln!(
                file,
                "-- Error dumping zone_notes (table may not exist): {}",
                e
            )
            .ok();
        }
    }

    // Dump zone_flags table
    writeln!(file, "\n-- Zone Flags table").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match sqlx::query("SELECT * FROM zone_flags ORDER BY id")
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(zone_flags) => {
            for flag in zone_flags {
                let id: i64 = flag.get("id");
                let zone_id: i64 = flag.get("zone_id");
                let flag_type_id: i64 = flag.get("flag_type_id");
                let created_at: String = flag.get("created_at");

                if let Err(_) = writeln!(
                    file,
                    "INSERT INTO zone_flags (id, zone_id, flag_type_id, created_at) VALUES ({}, {}, {}, '{}');",
                    id,
                    zone_id,
                    flag_type_id,
                    escape_sql(&created_at)
                ) {
                    error_count += 1;
                }
                total_records += 1;
            }
        }
        Err(e) => {
            writeln!(
                file,
                "-- Error dumping zone_flags (table may not exist): {}",
                e
            )
            .ok();
        }
    }

    // Dump links table
    writeln!(file, "\n-- Links table").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match sqlx::query("SELECT * FROM links ORDER BY id")
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(links) => {
            for link in links {
                let id: i64 = link.get("id");
                let name: String = link.get("name");
                let url: String = link.get("url");
                let category: String = link.get("category");
                let description: Option<String> = link.get("description");
                let created_at: String = link.get("created_at");
                let updated_at: String = link.get("updated_at");

                if let Err(_) = writeln!(
                    file,
                    "INSERT INTO links (id, name, url, category, description, created_at, updated_at) VALUES ({}, '{}', '{}', '{}', {}, '{}', '{}');",
                    id,
                    escape_sql(&name),
                    escape_sql(&url),
                    escape_sql(&category),
                    description
                        .map(|d| format!("'{}'", escape_sql(&d)))
                        .unwrap_or_else(|| "NULL".to_string()),
                    escape_sql(&created_at),
                    escape_sql(&updated_at)
                ) {
                    error_count += 1;
                }
                total_records += 1;
            }
        }
        Err(e) => {
            writeln!(file, "-- Error dumping links (table may not exist): {}", e).ok();
        }
    }

    // Dump instance_notes table if it exists
    writeln!(file, "\n-- Instance Notes table").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match sqlx::query("SELECT * FROM instance_notes ORDER BY id")
        .fetch_all(instance_pool.as_ref())
        .await
    {
        Ok(instance_notes) => {
            for note in instance_notes {
                let id: i64 = note.get("id");
                let instance_id: i64 = note.get("instance_id");
                let note_type_id: i64 = note.get("note_type_id");
                let content: String = note.get("content");
                let created_at: String = note.get("created_at");
                let updated_at: String = note.get("updated_at");

                if let Err(_) = writeln!(
                    file,
                    "INSERT INTO instance_notes (id, instance_id, note_type_id, content, created_at, updated_at) VALUES ({}, {}, {}, '{}', '{}', '{}');",
                    id,
                    instance_id,
                    note_type_id,
                    escape_sql(&content),
                    escape_sql(&created_at),
                    escape_sql(&updated_at)
                ) {
                    error_count += 1;
                }
                total_records += 1;
            }
        }
        Err(e) => {
            writeln!(
                file,
                "-- Error dumping instance_notes (table may not exist): {}",
                e
            )
            .ok();
        }
    }

    // Write footer
    writeln!(
        file,
        "\n-- Dump completed: {} records exported",
        total_records
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if error_count > 0 {
        writeln!(file, "-- Errors encountered: {}", error_count).ok();
    }

    drop(file); // Ensure file is closed

    println!(
        "Database dump created: {} ({} records, {} errors)",
        file_path, total_records, error_count
    );

    // Return HTML response with success message
    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Database Dump Complete - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .success {{ background: #d4edda; color: #155724; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
        .warning {{ background: #fff3cd; color: #856404; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
        .info {{ background: #d1ecf1; color: #0c5460; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
        .btn {{ background: #007bff; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px; }}
        .btn:hover {{ background: #0056b3; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/instances">Manage Instances</a>
    </div>

    <h1>Database Dump Complete</h1>

    <div class="{}">
        <h3>✅ Dump Successful</h3>
        <p><strong>File:</strong> {}</p>
        <p><strong>Records exported:</strong> {}</p>
        <p><strong>Timestamp:</strong> {}</p>
        {}
    </div>

    <div class="info">
        <h4>Exported Tables:</h4>
        <ul>
            <li>Zones</li>
            <li>Instances</li>
            <li>Zone Ratings</li>
            <li>Note Types</li>
            <li>Flag Types</li>
            <li>Zone Notes</li>
            <li>Zone Flags</li>
            <li>Links</li>
            <li>Instance Notes</li>
        </ul>
    </div>

    <a href="/admin" class="btn">Return to Admin Dashboard</a>
</body>
</html>
"#,
        if error_count > 0 {
            "warning"
        } else {
            "success"
        },
        file_path,
        total_records,
        now.format("%Y-%m-%d %H:%M:%S UTC"),
        if error_count > 0 {
            format!(
                "<p><strong>⚠️ Errors encountered:</strong> {} (check SQL file for details)</p>",
                error_count
            )
        } else {
            String::new()
        }
    );

    Ok(Html(html))
}
