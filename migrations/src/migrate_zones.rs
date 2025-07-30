use eq_rng::{get_zones_count, setup_database};
use serde::{Deserialize, Serialize};

use std::{fs, path::Path};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Zone {
    name: String,
    level_ranges: Vec<[u8; 2]>,
    expansion: String,
    continent: String,
    zone_type: String,
    connections: Vec<String>,
    image_url: String,
    map_url: String,
    rating: u8,
    hot_zone: bool,
    mission: bool,
    #[serde(default)]
    verified: bool,
}

pub async fn migrate_zones_to_db() -> Result<(), Box<dyn std::error::Error>> {
    // Setup database (creates tables if needed)
    let pool = setup_database().await?;

    // Check if zones table is already populated
    let count = get_zones_count(&pool).await?;

    if count > 0 {
        println!(
            "Database already contains {} zones. Skipping migration.",
            count
        );
        return Ok(());
    }

    println!("Loading zones from JSON files...");

    // Determine the correct data path - try both relative paths
    let data_path = if Path::new("data/zones").exists() {
        "data/zones"
    } else if Path::new("../data/zones").exists() {
        "../data/zones"
    } else {
        return Err("Could not find data/zones directory".into());
    };

    // List all zone JSON files
    let zone_files = [
        "classic.json",
        "kunark.json",
        "velious.json",
        "shadows_of_luclin.json",
        "planes_of_power.json",
        "loy.json",
        "ldon.json",
        "god.json",
        "god_missions.json",
        "oow.json",
        "oow_missions.json",
        "don.json",
        "don_missions.json",
        "dodh.json",
        "dodh_missions.json",
        "ro.json",
        "ro_missions.json",
        "tss.json",
        "tbs.json",
        "sof.json",
        "sof_missions.json",
        "sod.json",
        "sod_missions.json",
        "uf.json",
        "hot.json",
        "hot_missions.json",
        "voa.json",
        "rof.json",
        "rof_missions.json",
        "cotf.json",
        "tds.json",
        "tbm.json",
        "eok.json",
        "ros.json",
        "tbl.json",
        "tov.json",
        "cov.json",
        "tol.json",
        "nos.json",
        "ls.json",
        "ls_missions.json",
        "tob.json",
        "tob_missions.json",
    ];

    let mut all_zones: Vec<Zone> = Vec::new();

    // Load all zones from JSON files
    for file in zone_files {
        let full_path = format!("{}/{}", data_path, file);
        if let Ok(content) = fs::read_to_string(&full_path) {
            if let Ok(zones) = serde_json::from_str::<Vec<Zone>>(&content) {
                println!("Loaded {} zones from {}", zones.len(), full_path);
                all_zones.extend(zones);
            } else {
                eprintln!("Failed to parse JSON from {}", full_path);
            }
        } else {
            eprintln!("Failed to read file: {}", full_path);
        }
    }

    println!("Total zones loaded: {}", all_zones.len());

    // Insert zones into database
    println!("Inserting zones into database...");
    let mut transaction = pool.begin().await?;

    for zone in all_zones {
        let level_ranges_json = serde_json::to_string(&zone.level_ranges)?;
        let connections_json = serde_json::to_string(&zone.connections)?;

        sqlx::query(
            r#"
            INSERT INTO zones (
                name, level_ranges, expansion, continent, zone_type,
                connections, image_url, map_url, rating, hot_zone, mission, verified
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&zone.name)
        .bind(&level_ranges_json)
        .bind(&zone.expansion)
        .bind(&zone.continent)
        .bind(&zone.zone_type)
        .bind(&connections_json)
        .bind(&zone.image_url)
        .bind(&zone.map_url)
        .bind(zone.rating as i32)
        .bind(zone.hot_zone)
        .bind(zone.mission)
        .bind(zone.verified)
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;

    // Verify the migration
    let final_count = get_zones_count(&pool).await?;

    println!(
        "Migration completed successfully! {} zones inserted.",
        final_count
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    migrate_zones_to_db().await
}
