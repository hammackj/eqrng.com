# EQ RNG Migrations

This subcrate contains data migration utilities for the EverQuest RNG application.

## Purpose

The migrations crate is responsible for:
- Migrating zone data from JSON files to the SQLite database
- Future data migrations and transformations
- Database seeding and maintenance utilities

## Structure

- `src/lib.rs` - Main library interface
- `src/migrate_zones.rs` - Zone data migration functionality

## Usage

### Running Zone Migration

To migrate zones from JSON files to the database:

```bash
# From the root project directory
cargo run --bin migrate_zones --package eq_rng_migrations
```

Or from within the migrations directory:

```bash
cd migrations
cargo run --bin migrate_zones
```

### Using as a Library

The migration functions can also be used programmatically:

```rust
use eq_rng_migrations::migrate_zones_to_db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    migrate_zones_to_db().await?;
    Ok(())
}
```

## Dependencies

This crate depends on:
- The main `eq_rng` crate for database utilities
- `serde` and `serde_json` for JSON handling
- `sqlx` for database operations
- `tokio` for async runtime

## Data Sources

The migration reads zone data from JSON files in the `../data/zones/` directory, including:
- Classic zones
- All expansion zones (Kunark through latest)
- Mission zones
- Hot zones

## Database Location

The migration creates and populates the SQLite database at `../data/zones.db` (relative to the migrations directory) or `data/zones.db` (from the project root). This location allows the database to be:

- Shipped with application builds
- Included in Docker containers
- Easily backed up and version controlled
- Co-located with other data files

## Notes

- The migration will skip if zones already exist in the database
- Database is created at `data/zones.db` for easy deployment
- All migrations are wrapped in database transactions for safety
- Progress is logged to stdout during execution
- Migration works from both project root and migrations subdirectory