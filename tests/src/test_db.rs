use sqlx::{Row, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::path::Path;

/// Test database permissions and basic SQLite functionality
pub async fn test_database_permissions() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing database permissions and setup...");

    let database_url = "sqlite:./test.db";
    let db_path = "./test.db";

    // Test 1: Check if we can create a file in current directory
    println!("1. Testing file creation permissions...");
    match std::fs::File::create("test_permission.txt") {
        Ok(_) => {
            println!("   ✓ Can create files in current directory");
            std::fs::remove_file("test_permission.txt").ok();
        }
        Err(e) => {
            println!("   ✗ Cannot create files: {}", e);
            return Err(e.into());
        }
    }

    // Test 2: Check current working directory
    println!(
        "2. Current working directory: {:?}",
        std::env::current_dir()?
    );

    // Test 2.5: Check if data directory exists
    println!("2.5. Checking data directory structure...");
    if Path::new("data").exists() {
        println!("   ✓ data/ directory exists");
        if Path::new("data/zones").exists() {
            println!("   ✓ data/zones/ directory exists");
        }
        if Path::new("data/zones.db").exists() {
            println!("   ✓ data/zones.db exists");
        }
    } else {
        println!("   ⚠ data/ directory not found (normal for test environment)");
    }

    // Test 3: Try to create SQLite database
    println!("3. Testing SQLite database creation...");

    // Remove test database if it exists
    if Path::new(db_path).exists() {
        std::fs::remove_file(db_path)?;
        println!("   Removed existing test database");
    }

    // Create database
    match Sqlite::create_database(database_url).await {
        Ok(_) => println!("   ✓ Successfully created database"),
        Err(e) => {
            println!("   ✗ Failed to create database: {}", e);
            return Err(e.into());
        }
    }

    // Test 4: Connect to database
    println!("4. Testing database connection...");
    let pool = SqlitePool::connect(database_url).await?;
    println!("   ✓ Successfully connected to database");

    // Test 5: Create a simple table
    println!("5. Testing table creation...");
    sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
        .execute(&pool)
        .await?;
    println!("   ✓ Successfully created test table");

    // Test 6: Insert data
    println!("6. Testing data insertion...");
    sqlx::query("INSERT INTO test (name) VALUES ('test_data')")
        .execute(&pool)
        .await?;
    println!("   ✓ Successfully inserted test data");

    // Test 7: Query data
    println!("7. Testing data retrieval...");
    let row = sqlx::query("SELECT COUNT(*) as count FROM test")
        .fetch_one(&pool)
        .await?;
    let count: i64 = row.get("count");
    println!("   ✓ Successfully queried data, found {} rows", count);

    pool.close().await;

    // Cleanup
    if Path::new(db_path).exists() {
        std::fs::remove_file(db_path)?;
        println!("8. Cleaned up test database");
    }

    println!("\n✅ All database tests passed! Your system should work fine.");
    Ok(())
}

/// Test the main application database
pub async fn test_app_database() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nTesting main application database...");

    // Check if we can connect to the app database
    let app_db_path = "data/zones.db";
    if Path::new(app_db_path).exists() {
        println!("1. Main database exists at {}", app_db_path);

        let database_url = format!("sqlite:./{}", app_db_path);
        match SqlitePool::connect(&database_url).await {
            Ok(pool) => {
                println!("   ✓ Successfully connected to main database");

                // Test zone count
                let row = sqlx::query("SELECT COUNT(*) as count FROM zones")
                    .fetch_one(&pool)
                    .await?;
                let count: i64 = row.get("count");
                println!("   ✓ Found {} zones in database", count);

                // Test a sample query
                let row =
                    sqlx::query("SELECT name, expansion FROM zones ORDER BY RANDOM() LIMIT 1")
                        .fetch_one(&pool)
                        .await?;
                let name: String = row.get("name");
                let expansion: String = row.get("expansion");
                println!("   ✓ Sample zone: '{}' from {}", name, expansion);

                pool.close().await;
            }
            Err(e) => {
                println!("   ✗ Failed to connect to main database: {}", e);
                return Err(e.into());
            }
        }
    } else {
        println!(
            "1. Main database not found at {} (run migrations first)",
            app_db_path
        );
    }

    Ok(())
}

/// Run all database tests
pub async fn run_all_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Running EQ RNG Database Tests\n");

    // Test 1: Basic database permissions
    test_database_permissions().await?;

    // Test 2: Main application database
    test_app_database().await?;

    println!("\n🎉 All tests completed successfully!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_all_tests().await
}
