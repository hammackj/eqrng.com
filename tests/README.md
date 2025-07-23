# EQ RNG Tests

This subcrate contains testing utilities and database validation tools for the EverQuest RNG application.

## Purpose

The tests crate is responsible for:
- Database connectivity and permission testing
- Application database validation
- Development and deployment environment testing
- Database health checks and diagnostics

## Structure

- `src/lib.rs` - Main library interface and utilities
- `src/test_db.rs` - Database testing functionality

## Usage

### Running Database Tests

To run all database tests:

```bash
# From the root project directory
cargo run --bin test_db --package eq_rng_tests

# Or from within the tests directory
cd tests
cargo run --bin test_db
```

### Test Categories

#### 1. Basic Database Permissions
- Tests file creation permissions in current directory
- Verifies SQLite database creation capabilities
- Tests basic SQL operations (CREATE, INSERT, SELECT)
- Validates database cleanup

#### 2. Application Database Validation
- Checks for existence of main application database (`data/zones.db`)
- Validates database connectivity
- Counts zones in the database
- Performs sample queries to verify data integrity

### Using as a Library

The test functions can also be used programmatically:

```rust
use eq_rng_tests::{run_all_tests, test_database_permissions, test_app_database};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Run all tests
    run_all_tests().await?;
    
    // Or run individual test suites
    test_database_permissions().await?;
    test_app_database().await?;
    
    Ok(())
}
```

### Utility Functions

The crate also provides utility functions:

```rust
use eq_rng_tests::utils::{file_exists_and_readable, current_dir_string};

// Check if a file exists and is readable
if file_exists_and_readable("data/zones.db") {
    println!("Database file is accessible");
}

// Get current working directory
let cwd = current_dir_string()?;
println!("Working in: {}", cwd);
```

## Dependencies

This crate depends on:
- The main `eq_rng` crate for database utilities
- `sqlx` for database operations
- `tokio` for async runtime

## Test Output

The tests provide detailed output showing:
- ✓ Successful operations
- ✗ Failed operations
- ⚠ Warnings or informational messages

Example output:
```
🧪 Running EQ RNG Database Tests

Testing database permissions and setup...
1. Testing file creation permissions...
   ✓ Can create files in current directory
2. Current working directory: "/path/to/eq_rng"
2.5. Checking data directory structure...
   ✓ data/ directory exists
   ✓ data/zones/ directory exists
   ✓ data/zones.db exists
...

Testing main application database...
1. Main database exists at data/zones.db
   ✓ Successfully connected to main database
   ✓ Found 703 zones in database
   ✓ Sample zone: 'Befallen' from Classic

🎉 All tests completed successfully!
```

## When to Run

- **Before deployment** - Verify environment is properly configured
- **After migrations** - Validate database was populated correctly
- **Troubleshooting** - Diagnose database connectivity issues
- **Development setup** - Ensure local environment is working

## Notes

- Tests create temporary files that are automatically cleaned up
- The main database tests are non-destructive (read-only)
- Tests work from both project root and tests subdirectory
- All async operations use proper error handling