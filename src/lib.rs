use sqlx::{Row, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::path::Path;

pub mod admin;
pub mod classes;
pub mod instances;
pub mod links;
pub mod races;
pub mod ratings;
pub mod version;
pub mod zones;

// Anonymize IP migration utilities
use blake3;
use chrono::Utc;
use std::env;

// Reuse the same keyed blake3 scheme as ratings.rs
fn rating_ip_hash_key() -> [u8; 32] {
    let key_material = env::var("RATING_IP_HASH_KEY")
        .unwrap_or_else(|_| String::from("eq_rng_default_rating_ip_key_v1"));
    let hash = blake3::hash(key_material.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(hash.as_bytes());
    key
}

fn hash_ip(ip: &str) -> String {
    let key = rating_ip_hash_key();
    let h = blake3::keyed_hash(&key, ip.as_bytes());
    h.to_hex().to_string()
}

fn is_lower_hex64(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

// Startup migration to hash any existing plaintext IPs in zone_ratings
pub async fn migrate_hash_zone_ratings(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Ensure table exists
    let table_exists =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='zone_ratings'")
            .fetch_optional(pool)
            .await?
            .is_some();
    if !table_exists {
        return Ok(());
    }

    // Check if there are any rows that don't look hashed yet
    let unhashed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM zone_ratings WHERE NOT (length(user_ip) = 64 AND user_ip GLOB '[0-9a-f]*')",
    )
    .fetch_one(pool)
    .await?;

    if unhashed_count == 0 {
        return Ok(());
    }

    println!("Anonymizing {} existing rating IP(s)...", unhashed_count);

    let mut tx = pool.begin().await?;

    // Load all existing rows
    let rows =
        sqlx::query("SELECT zone_id, user_ip, rating, created_at, updated_at FROM zone_ratings")
            .fetch_all(&mut *tx)
            .await?;

    // Clear table and re-insert with hashed IPs, merging on conflict
    sqlx::query("DELETE FROM zone_ratings")
        .execute(&mut *tx)
        .await?;

    for row in rows {
        let zone_id: i64 = row.get("zone_id");
        let user_ip: String = row.get("user_ip");
        let rating: i32 = row.get("rating");
        let created_at: Option<String> = row.get("created_at");
        let updated_at: Option<String> = row.get("updated_at");

        let ip_hashed = if is_lower_hex64(&user_ip) {
            user_ip
        } else {
            hash_ip(&user_ip)
        };

        let created_at_val = created_at.unwrap_or_else(|| Utc::now().to_rfc3339());
        let updated_at_val = updated_at.unwrap_or_else(|| Utc::now().to_rfc3339());

        sqlx::query(
            r#"
            INSERT INTO zone_ratings (zone_id, user_ip, rating, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(zone_id, user_ip) DO UPDATE SET
                rating = excluded.rating,
                updated_at = excluded.updated_at,
                created_at = CASE
                    WHEN zone_ratings.created_at < excluded.created_at THEN zone_ratings.created_at
                    ELSE excluded.created_at
                END
            "#,
        )
        .bind(zone_id)
        .bind(ip_hashed)
        .bind(rating)
        .bind(created_at_val)
        .bind(updated_at_val)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    println!("IP anonymization complete.");
    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    pub zone_state: zones::ZoneState,
    pub instance_state: instances::InstanceState,
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

    // Check if data.sql exists - if so, load from it instead of creating tables
    if std::path::Path::new("./data/data.sql").exists() {
        println!("Found data.sql, loading from file...");
        load_data_sql(&pool).await?;
    } else {
        println!("No data.sql found, creating tables from scratch...");
        // Run migrations to create basic table structure
        create_tables(&pool).await?;
    }

    // Run database migrations
    if let Err(e) = run_migrations(&pool).await {
        eprintln!("Warning: failed to run migrations: {}", e);
    }

    // Anonymize any existing plaintext IPs in ratings before continuing
    if let Err(e) = migrate_hash_zone_ratings(&pool).await {
        eprintln!("Warning: failed to hash existing rating IPs: {}", e);
    }

    // Force WAL checkpoint to consolidate changes into main database file
    checkpoint_wal(&pool).await?;

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
                verified BOOLEAN NOT NULL DEFAULT FALSE,
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
            "CREATE INDEX IF NOT EXISTS idx_zones_hot_zone ON zones(hot_zone)",
            "CREATE INDEX IF NOT EXISTS idx_zones_rating ON zones(rating)",
            "CREATE INDEX IF NOT EXISTS idx_zones_continent ON zones(continent)",
            "CREATE INDEX IF NOT EXISTS idx_zones_verified ON zones(verified)",
        ];

        for index_sql in &indexes {
            sqlx::query(index_sql).execute(pool).await?;
        }

        println!("Zones table and indexes created successfully");
    } else {
        println!("Zones table already exists");

        // Check if verified column exists, if not add it
        let verified_column_exists =
            sqlx::query("SELECT name FROM pragma_table_info('zones') WHERE name='verified'")
                .fetch_optional(pool)
                .await?
                .is_some();

        if !verified_column_exists {
            println!("Adding verified column to zones table...");
            sqlx::query("ALTER TABLE zones ADD COLUMN verified BOOLEAN NOT NULL DEFAULT FALSE")
                .execute(pool)
                .await?;

            sqlx::query("CREATE INDEX IF NOT EXISTS idx_zones_verified ON zones(verified)")
                .execute(pool)
                .await?;

            println!("Verified column added successfully");
        }

        // Mission column migration disabled for fresh database creation
        // The table schema above already excludes the mission column
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

    // Check if instances table exists
    let instances_table_exists =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='instances'")
            .fetch_optional(pool)
            .await?
            .is_some();

    if !instances_table_exists {
        println!("Creating instances table...");

        sqlx::query(
            r#"
            CREATE TABLE instances (
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
                verified BOOLEAN NOT NULL DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create indexes
        let instance_indexes = [
            "CREATE INDEX IF NOT EXISTS idx_instances_expansion ON instances(expansion)",
            "CREATE INDEX IF NOT EXISTS idx_instances_zone_type ON instances(zone_type)",
            "CREATE INDEX IF NOT EXISTS idx_instances_hot_zone ON instances(hot_zone)",
            "CREATE INDEX IF NOT EXISTS idx_instances_rating ON instances(rating)",
            "CREATE INDEX IF NOT EXISTS idx_instances_continent ON instances(continent)",
            "CREATE INDEX IF NOT EXISTS idx_instances_verified ON instances(verified)",
        ];

        for index_sql in &instance_indexes {
            sqlx::query(index_sql).execute(pool).await?;
        }

        println!("Instances table and indexes created successfully");
    } else {
        println!("Instances table already exists");

        // Mission column migration disabled for fresh database creation
        // The table schema above already excludes the mission column
    }

    // Check if instance_notes table exists
    let instance_notes_table_exists =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='instance_notes'")
            .fetch_optional(pool)
            .await?
            .is_some();

    if !instance_notes_table_exists {
        println!("Creating instance_notes table...");

        sqlx::query(
            r#"
            CREATE TABLE instance_notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                instance_id INTEGER NOT NULL,
                note_type_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (instance_id) REFERENCES instances (id) ON DELETE CASCADE,
                FOREIGN KEY (note_type_id) REFERENCES note_types (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create indexes for instance_notes
        let instance_note_indexes = [
            "CREATE INDEX IF NOT EXISTS idx_instance_notes_instance_id ON instance_notes(instance_id)",
            "CREATE INDEX IF NOT EXISTS idx_instance_notes_note_type_id ON instance_notes(note_type_id)",
            "CREATE INDEX IF NOT EXISTS idx_instance_notes_created_at ON instance_notes(created_at)",
        ];

        for index_sql in &instance_note_indexes {
            sqlx::query(index_sql).execute(pool).await?;
        }

        println!("Instance notes table and indexes created successfully");
    } else {
        println!("Instance notes table already exists");
    }

    // Check if flag_types table exists
    let flag_types_table_exists =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='flag_types'")
            .fetch_optional(pool)
            .await?
            .is_some();

    if !flag_types_table_exists {
        println!("Creating flag_types table...");

        sqlx::query(
            r#"
            CREATE TABLE flag_types (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                color_class TEXT NOT NULL DEFAULT 'bg-blue-500',
                filterable BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Insert default flag types
        let default_flag_types = [
            ("hot_zone", "Hot Zone", "bg-red-500", true),
            ("undead", "Undead", "bg-purple-500", true),
        ];

        for (name, display_name, color_class, filterable) in &default_flag_types {
            sqlx::query(
                "INSERT INTO flag_types (name, display_name, color_class, filterable) VALUES (?, ?, ?, ?)",
            )
            .bind(name)
            .bind(display_name)
            .bind(color_class)
            .bind(*filterable)
            .execute(pool)
            .await?;
        }

        println!("Flag types table created successfully with default types");
    } else {
        println!("Flag types table already exists");
    }

    // Check if zone_flags table exists
    let zone_flags_table_exists =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='zone_flags'")
            .fetch_optional(pool)
            .await?
            .is_some();

    if !zone_flags_table_exists {
        println!("Creating zone_flags table...");

        sqlx::query(
            r#"
            CREATE TABLE zone_flags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                zone_id INTEGER NOT NULL,
                flag_type_id INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (zone_id) REFERENCES zones (id) ON DELETE CASCADE,
                FOREIGN KEY (flag_type_id) REFERENCES flag_types (id) ON DELETE CASCADE,
                UNIQUE(zone_id, flag_type_id)
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create indexes for zone_flags
        let flag_indexes = [
            "CREATE INDEX IF NOT EXISTS idx_zone_flags_zone_id ON zone_flags(zone_id)",
            "CREATE INDEX IF NOT EXISTS idx_zone_flags_flag_type_id ON zone_flags(flag_type_id)",
            "CREATE INDEX IF NOT EXISTS idx_zone_flags_created_at ON zone_flags(created_at)",
        ];

        for index_sql in &flag_indexes {
            sqlx::query(index_sql).execute(pool).await?;
        }

        println!("Zone flags table and indexes created successfully");
    } else {
        println!("Zone flags table already exists");
    }

    Ok(())
}

pub async fn get_zones_count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM zones")
        .fetch_one(pool)
        .await?;

    Ok(row.get("count"))
}

pub async fn get_instances_count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM instances")
        .fetch_one(pool)
        .await?;

    Ok(row.get("count"))
}

pub async fn checkpoint_wal(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Force WAL checkpoint to consolidate all changes into main database file
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(pool)
        .await?;
    println!("WAL checkpoint completed - changes consolidated to main database file");
    Ok(())
}

pub async fn database_health_check(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Simple query to check if database is working
    sqlx::query("SELECT 1").fetch_one(pool).await?;
    Ok(())
}

/// Create migration tracking table if it doesn't exist
async fn create_migration_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Check if data.sql needs to be loaded based on file modification time
async fn should_load_data_sql(pool: &SqlitePool) -> Result<bool, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;

    // Check if data.sql exists
    let data_sql_path = "./data/data.sql";
    if !Path::new(data_sql_path).exists() {
        println!("data.sql not found, skipping data migration");
        return Ok(false);
    }

    // Get file modification time
    let metadata = fs::metadata(data_sql_path)?;
    let file_modified = metadata.modified()?;
    let file_modified_timestamp = file_modified
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    // Check if we've already loaded this version
    let last_migration = sqlx::query(
        "SELECT applied_at FROM migrations WHERE name = 'data_sql' ORDER BY applied_at DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = last_migration {
        let applied_at: String = row.get("applied_at");
        // Parse the SQLite datetime string to compare with file timestamp
        let applied_timestamp = chrono::DateTime::parse_from_str(
            &format!("{}+00:00", applied_at),
            "%Y-%m-%d %H:%M:%S%z",
        )
        .map_err(|e| format!("Failed to parse migration timestamp: {}", e))?
        .timestamp();

        // If file is newer than last migration, we need to reload
        Ok(file_modified_timestamp > applied_timestamp)
    } else {
        // No previous migration, we should load
        Ok(true)
    }
}

/// Load data from data.sql file if it's newer than the last migration
pub async fn load_data_sql(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // Create migration tracking table first
    create_migration_table(pool).await?;

    // Check if we should load data.sql
    if !should_load_data_sql(pool).await? {
        println!("data.sql is up to date, skipping migration");
        return Ok(());
    }

    println!("Loading data from data.sql...");

    // Read and process the SQL file manually to handle schema differences
    let sql_content = fs::read_to_string("./data/data.sql")?;

    // Start a transaction to ensure atomicity
    let mut transaction = pool.begin().await?;

    // Drop all existing tables to start fresh
    let drop_tables = [
        "DROP TABLE IF EXISTS zone_flags",
        "DROP TABLE IF EXISTS zone_notes",
        "DROP TABLE IF EXISTS instance_notes",
        "DROP TABLE IF EXISTS zone_ratings",
        "DROP TABLE IF EXISTS instances",
        "DROP TABLE IF EXISTS zones",
        "DROP TABLE IF EXISTS flag_types",
        "DROP TABLE IF EXISTS note_types",
        "DROP TABLE IF EXISTS links",
    ];

    for drop_sql in &drop_tables {
        sqlx::query(drop_sql).execute(&mut *transaction).await?;
    }

    // Process the SQL content line by line
    let lines: Vec<&str> = sql_content.lines().collect();
    let mut current_statement = String::new();

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }

        // Skip PRAGMA statements and transaction statements that might cause issues
        if trimmed.starts_with("PRAGMA")
            || trimmed.starts_with("BEGIN")
            || trimmed.starts_with("COMMIT")
        {
            continue;
        }

        current_statement.push_str(line);
        current_statement.push(' ');

        // Execute when we hit a semicolon
        if trimmed.ends_with(';') {
            let statement = current_statement.trim();
            if !statement.is_empty() {
                match sqlx::query(statement).execute(&mut *transaction).await {
                    Ok(_) => {}
                    Err(e) => {
                        let error_message = e.to_string();

                        // Skip harmless errors that are expected during data.sql loading
                        if error_message.contains("table migrations already exists")
                            || error_message.contains("UNIQUE constraint failed: migrations.id")
                        {
                            // These are expected - ignore silently
                        } else {
                            // Log unexpected errors but continue
                            eprintln!("Warning: Failed to execute statement: {}", e);
                            eprintln!("Statement was: {}", statement);
                        }
                    }
                }
            }
            current_statement.clear();
        }
    }

    // Commit the transaction
    transaction.commit().await?;

    // Record this migration
    sqlx::query("INSERT OR REPLACE INTO migrations (name) VALUES ('data_sql')")
        .execute(pool)
        .await?;

    println!("Successfully loaded data from data.sql");
    Ok(())
}

/// Export current database to a timestamped data.sql file
pub async fn dump_database_to_sql(
    _pool: &SqlitePool,
) -> Result<String, Box<dyn std::error::Error>> {
    use chrono::Utc;
    use std::process::Command;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("data-{}.sql", timestamp);
    let filepath = format!("./data/{}", filename);

    println!("Dumping database to {}...", filepath);

    let output = Command::new("sqlite3")
        .arg("./data/zones.db")
        .arg(".dump")
        .output()?;

    if output.status.success() {
        std::fs::write(&filepath, output.stdout)?;
        println!("Database successfully dumped to {}", filepath);
        Ok(filename)
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to dump database: {}", error).into())
    }
}

pub async fn migrate_hot_zones_to_flags(pool: &SqlitePool) -> Result<i32, sqlx::Error> {
    // Migration function to convert existing hot_zone boolean values to zone flags
    // Returns the number of zones migrated

    println!("Starting hot zone to flags migration...");

    // Get the flag_type_id for hot_zone flag
    let flag_type_row = sqlx::query("SELECT id FROM flag_types WHERE name = 'hot_zone'")
        .fetch_optional(pool)
        .await?;

    let flag_type_id = match flag_type_row {
        Some(row) => row.get::<i64, _>("id"),
        None => {
            println!("Warning: hot_zone flag type not found, skipping migration");
            return Ok(0);
        }
    };

    // Count zones that need migration (hot_zone = true but no flag)
    let count_row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM zones z
        WHERE z.hot_zone = 1
        AND NOT EXISTS (
            SELECT 1 FROM zone_flags zf
            WHERE zf.zone_id = z.id AND zf.flag_type_id = ?
        )
        "#,
    )
    .bind(flag_type_id)
    .fetch_one(pool)
    .await?;

    let zones_to_migrate: i32 = count_row.get("count");

    if zones_to_migrate == 0 {
        println!("No zones need migration - all hot zones already have flags");
        return Ok(0);
    }

    println!(
        "Found {} zones with hot_zone = true that need flag migration",
        zones_to_migrate
    );

    // Perform the migration
    let result = sqlx::query(
        r#"
        INSERT OR IGNORE INTO zone_flags (zone_id, flag_type_id)
        SELECT z.id, ?
        FROM zones z
        WHERE z.hot_zone = 1
        AND NOT EXISTS (
            SELECT 1 FROM zone_flags zf
            WHERE zf.zone_id = z.id AND zf.flag_type_id = ?
        )
        "#,
    )
    .bind(flag_type_id)
    .bind(flag_type_id)
    .execute(pool)
    .await?;

    let migrated_count = result.rows_affected() as i32;

    // Verify migration
    let verification_row = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM zone_flags zf
        JOIN flag_types ft ON zf.flag_type_id = ft.id
        WHERE ft.name = 'hot_zone'
        "#,
    )
    .fetch_one(pool)
    .await?;

    let total_flags: i32 = verification_row.get("count");

    println!(
        "Migration completed: {} zones migrated, {} total hot zone flags now exist",
        migrated_count, total_flags
    );

    // Force WAL checkpoint to consolidate changes
    checkpoint_wal(pool).await?;

    Ok(migrated_count)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    println!("Running database migrations...");

    // Migration 1: Add filterable column to flag_types
    let filterable_column_exists =
        sqlx::query("SELECT * FROM pragma_table_info('flag_types') WHERE name='filterable'")
            .fetch_optional(pool)
            .await?
            .is_some();

    if !filterable_column_exists {
        println!("Adding filterable column to flag_types table...");
        sqlx::query("ALTER TABLE flag_types ADD COLUMN filterable BOOLEAN NOT NULL DEFAULT 1")
            .execute(pool)
            .await?;
        println!("Filterable column added successfully");
    } else {
        println!("Filterable column already exists in flag_types table");
    }

    println!("Database migrations completed successfully");
    Ok(())
}
