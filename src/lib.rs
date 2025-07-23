use sqlx::{Row, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::path::Path;

pub mod admin;
pub mod classes;
pub mod links;
pub mod races;
pub mod ratings;
pub mod version;
pub mod zones;

#[derive(Clone)]
pub struct AppState {
    pub zone_state: zones::ZoneState,
    pub class_race_state: classes::ClassRaceState,
}

pub async fn setup_database() -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let database_url = "sqlite:./data/zones.db";
    let db_path = "./data/zones.db";

    // Check if database file exists, if not create it
    if !Path::new(db_path).exists() {
        println!("Database file doesn't exist, creating...");
        Sqlite::create_database(database_url).await?;
    }

    // Connect to database
    let pool = SqlitePool::connect(database_url).await?;

    // Run migrations
    create_tables(&pool).await?;

    println!("Database setup completed successfully");
    Ok(pool)
}

async fn create_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Check if zones table exists
    let table_exists =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='zones'")
            .fetch_optional(pool)
            .await?
            .is_some();

    if !table_exists {
        println!("Creating zones table...");

        sqlx::query(
            r#"
            CREATE TABLE zones (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                level_ranges TEXT NOT NULL,
                expansion TEXT NOT NULL,
                continent TEXT NOT NULL DEFAULT '',
                zone_type TEXT NOT NULL,
                connections TEXT NOT NULL DEFAULT '[]',
                image_url TEXT NOT NULL DEFAULT '',
                map_url TEXT NOT NULL DEFAULT '',
                rating INTEGER NOT NULL DEFAULT 0,
                hot_zone BOOLEAN NOT NULL DEFAULT FALSE,
                mission BOOLEAN NOT NULL DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create indexes
        let indexes = [
            "CREATE INDEX IF NOT EXISTS idx_zones_expansion ON zones(expansion)",
            "CREATE INDEX IF NOT EXISTS idx_zones_zone_type ON zones(zone_type)",
            "CREATE INDEX IF NOT EXISTS idx_zones_mission ON zones(mission)",
            "CREATE INDEX IF NOT EXISTS idx_zones_hot_zone ON zones(hot_zone)",
            "CREATE INDEX IF NOT EXISTS idx_zones_rating ON zones(rating)",
            "CREATE INDEX IF NOT EXISTS idx_zones_continent ON zones(continent)",
        ];

        for index_sql in &indexes {
            sqlx::query(index_sql).execute(pool).await?;
        }

        println!("Zones table and indexes created successfully");
    } else {
        println!("Zones table already exists");
    }

    // Check if zone_ratings table exists
    let ratings_table_exists =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='zone_ratings'")
            .fetch_optional(pool)
            .await?
            .is_some();

    if !ratings_table_exists {
        println!("Creating zone_ratings table...");

        sqlx::query(
            r#"
            CREATE TABLE zone_ratings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                zone_id INTEGER NOT NULL,
                user_ip TEXT NOT NULL,
                rating INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (zone_id) REFERENCES zones (id) ON DELETE CASCADE,
                UNIQUE(zone_id, user_ip)
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create indexes for zone_ratings
        let rating_indexes = [
            "CREATE INDEX IF NOT EXISTS idx_zone_ratings_zone_id ON zone_ratings(zone_id)",
            "CREATE INDEX IF NOT EXISTS idx_zone_ratings_rating ON zone_ratings(rating)",
            "CREATE INDEX IF NOT EXISTS idx_zone_ratings_created_at ON zone_ratings(created_at)",
        ];

        for index_sql in &rating_indexes {
            sqlx::query(index_sql).execute(pool).await?;
        }

        println!("Zone ratings table and indexes created successfully");
    } else {
        println!("Zone ratings table already exists");
    }

    // Check if note_types table exists
    let note_types_table_exists =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='note_types'")
            .fetch_optional(pool)
            .await?
            .is_some();

    if !note_types_table_exists {
        println!("Creating note_types table...");

        sqlx::query(
            r#"
            CREATE TABLE note_types (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                color_class TEXT NOT NULL DEFAULT 'bg-blue-500',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Insert default note types
        let default_note_types = [
            ("epic_1_0", "Epic 1.0", "bg-yellow-500"),
            ("epic_1_5", "Epic 1.5", "bg-orange-500"),
            ("epic_2_0", "Epic 2.0", "bg-red-500"),
            ("zone_aug", "Zone Aug", "bg-purple-500"),
        ];

        for (name, display_name, color_class) in &default_note_types {
            sqlx::query(
                "INSERT INTO note_types (name, display_name, color_class) VALUES (?, ?, ?)",
            )
            .bind(name)
            .bind(display_name)
            .bind(color_class)
            .execute(pool)
            .await?;
        }

        println!("Note types table created successfully with default types");
    } else {
        println!("Note types table already exists");
    }

    // Check if zone_notes table exists
    let zone_notes_table_exists =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='zone_notes'")
            .fetch_optional(pool)
            .await?
            .is_some();

    if !zone_notes_table_exists {
        println!("Creating zone_notes table...");

        sqlx::query(
            r#"
            CREATE TABLE zone_notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                zone_id INTEGER NOT NULL,
                note_type_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (zone_id) REFERENCES zones (id) ON DELETE CASCADE,
                FOREIGN KEY (note_type_id) REFERENCES note_types (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create indexes for zone_notes
        let note_indexes = [
            "CREATE INDEX IF NOT EXISTS idx_zone_notes_zone_id ON zone_notes(zone_id)",
            "CREATE INDEX IF NOT EXISTS idx_zone_notes_note_type_id ON zone_notes(note_type_id)",
            "CREATE INDEX IF NOT EXISTS idx_zone_notes_created_at ON zone_notes(created_at)",
        ];

        for index_sql in &note_indexes {
            sqlx::query(index_sql).execute(pool).await?;
        }

        println!("Zone notes table and indexes created successfully");
    } else {
        println!("Zone notes table already exists");
    }

    // Check if links table exists
    let links_table_exists =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='links'")
            .fetch_optional(pool)
            .await?
            .is_some();

    if !links_table_exists {
        println!("Creating links table...");

        sqlx::query(
            r#"
            CREATE TABLE links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create indexes for links
        let link_indexes = [
            "CREATE INDEX IF NOT EXISTS idx_links_category ON links(category)",
            "CREATE INDEX IF NOT EXISTS idx_links_name ON links(name)",
            "CREATE INDEX IF NOT EXISTS idx_links_created_at ON links(created_at)",
        ];

        for index_sql in &link_indexes {
            sqlx::query(index_sql).execute(pool).await?;
        }

        println!("Links table and indexes created successfully");
    } else {
        println!("Links table already exists");
    }

    Ok(())
}

pub async fn get_zones_count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM zones")
        .fetch_one(pool)
        .await?;

    Ok(row.get("count"))
}

pub async fn database_health_check(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Simple query to check if database is working
    sqlx::query("SELECT 1").fetch_one(pool).await?;
    Ok(())
}
