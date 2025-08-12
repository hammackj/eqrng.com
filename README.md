# EQRng.com

Server runs on port 3000 this is exported from docker to the host machine. The host machine is running nginx and proxies 3000 to 80/443. Currently the frontend is a dev test front end. I plan on making a proper frontend once I have all the features implemented.

## Project Structure

This is a Rust workspace with the following components:

### 📦 Packages

- **`eq_rng`** (`src/`) - Main web server and API endpoints


### 📁 Directory Layout

```
eq_rng.com/
├── src/                    # Main application source
│   ├── main.rs            # Web server entry point
│   ├── lib.rs             # Database setup and utilities
│   ├── zones.rs           # Zone API endpoints
│   ├── instances.rs       # Instance API endpoints
│   ├── classes.rs         # Class API endpoints
│   ├── races.rs           # Race API endpoints
│   ├── ratings.rs         # Rating API endpoints
│   ├── links.rs           # Links API endpoints
│   ├── admin.rs           # Admin interface (optional feature)
│   └── version.rs         # Version API endpoint
├── data/                  # Data files and database
│   ├── data.sql          # Database source of truth (SQL dump)
│   ├── class_race.json   # Class/race compatibility data
├── dist/                  # Frontend build output
└── Cargo.toml            # Workspace configuration
```

### 🔧 Available Binaries

- **`eq_rng`** - Main web server application
- **`test_db`** - Database testing and validation tool

## API Endpoints

### /random_zone
#### Parameters
- min_level
- max_level
- zone_type
- expansion
- continent
- flags (comma-separated list, e.g., "hot_zone,undead")

This endpoint will return a random zone based on the given parameters.

### /random_instance
#### Parameters
- min_level
- max_level
- zone_type
- expansion
- continent
- hot_zone

Returns a random instance based on the given parameters.

### /random_class
#### Parameters
- race (optional)

Returns a random class, optionally filtered by race compatibility.

### /random_race

Returns a random race with gender and image information.

### /version

Returns the current application version.

### Zone Ratings
- `GET /zones/:zone_id/rating` - Get average rating for a zone
- `POST /zones/:zone_id/rating` - Submit a rating for a zone
- `GET /zones/:zone_id/ratings` - Get all ratings for a zone
- `DELETE /api/ratings/:id` - Delete a specific rating

### Rating Transaction Log
- File-based transaction logging to `data/rating_transaction.log`
- Extract and apply via utility scripts (no API endpoints for security)

### Zone Notes
- `GET /zones/:zone_id/notes` - Get notes for a zone
- `GET /instances/:instance_id/notes` - Get notes for an instance

### Links API
- `GET /api/links` - Get all links
- `GET /api/links/by-category` - Get links grouped by category
- `GET /api/links/categories` - Get all link categories
- `POST /api/links` - Create a new link
- `GET /api/links/:id` - Get a specific link
- `PUT /api/links/:id` - Update a link
- `DELETE /api/links/:id` - Delete a link

## Data Management

### New Data.sql Migration System

The application now uses a **data.sql file** as the single source of truth for all database content. This replaces the previous JSON-based migration system.

#### Key Benefits
- **Version Control Friendly**: SQL files can be easily reviewed in PRs
- **Hand Editable**: Direct SQL editing for data changes
- **Simplified Deployment**: No binary database files to ship
- **Atomic Updates**: All changes applied in single transaction
- **Backup System**: Admin interface creates timestamped dumps

#### How It Works
1. **On startup**: Application checks if `data/data.sql` exists
2. **If newer**: Compares file timestamp vs last migration
3. **If updated**: Drops all tables and loads fresh from `data.sql`
4. **Tracking**: Records migration timestamp in `migrations` table

#### File Locations
- **`data/data.sql`** - Current database content (ships with build)
- **`data/zones.db`** - Generated SQLite database file
- **`data/data-YYYYMMDD_HHMMSS.sql`** - Timestamped backups from admin interface

### Database Structure

The database contains the following main tables:
- **zones** - Zone information (386 zones across all expansions)
- **instances** - Instance information (moved from zones)
- **zone_ratings** - User ratings for zones
- **data/rating_transaction.log** - File-based audit trail of all rating operations
- **zone_notes** - Categorized notes for zones
- **instance_notes** - Categorized notes for instances
- **note_types** - Note categories (Epic 1.0, Epic 1.5, Epic 2.0, Zone Aug)
- **flag_types** - Flag categories for zones
- **zone_flags** - Zone flag associations
- **links** - External links organized by category
- **migrations** - Migration tracking

### Making Data Changes

#### Option A: Direct SQL Editing
```bash
# Edit the SQL file directly
vim data/data.sql

# Restart application to load changes
cargo run --features admin
```

#### Option B: Admin Interface
```bash
# Start the application
cargo run --features admin

# Make changes via admin interface at /admin
# Then create a new dump
curl -X POST http://localhost:3000/admin/dump-database

# Replace data.sql with the new dump
mv data/data-YYYYMMDD_HHMMSS.sql data/data.sql
```

### Class/Race Data

The class/race compatibility data remains in JSON format:
- **`data/class_race.json`** - Defines which classes each race can be
- Used by the `/random_class?race=RaceName` endpoint
- Contains EverQuest game rules for race/class combinations

## Testing

The project includes a separate tests subcrate for database validation:

### Running Tests

```bash
# Using the test script (recommended)
./run_tests.sh db          # Database tests only
./run_tests.sh build       # Build tests only
./run_tests.sh all         # All test suites

# Or directly with cargo
cargo run --bin test_db --package eq_rng_tests
```

### Test Features

- Database connectivity and permission validation
- Main application database integrity checks
- Non-destructive testing (read-only operations)
- Detailed test output with success/failure indicators

See `tests/README.md` for more details.

## Development

### Building

The project supports multiple build configurations for different environments:

#### Using the Build Script (Recommended)
```bash
# Production build (no admin interface)
./build.sh production

# Development build (with admin interface)
./build.sh development

# Local build (with admin interface)
./build.sh local
```

#### Manual Building
```bash
# Production build (no admin interface)
cargo build --release --no-default-features

# Development build (with admin interface)
cargo build --release --features admin

# Regular build
cargo build
```

### Running the server
```bash
# Local development with admin interface
cargo run --bin eq_rng --features admin

# Production mode (no admin interface)
cargo run --bin eq_rng --no-default-features

# Regular run
cargo run --bin eq_rng
```

### Database Setup

For fresh environments, the database is automatically created from `data/data.sql`:

```bash
# Clone repository
git clone <repository-url>
cd eq_rng

# Start application (database auto-created from data.sql)
cargo run --features admin

# Check database was created
sqlite3 data/zones.db "SELECT COUNT(*) FROM zones;"
```

### Running tests
```bash
# Run database tests
./run_tests.sh db

# Run build tests
./run_tests.sh build

# Run all tests
./run_tests.sh all
```

## Quick Command Reference

### 🚀 Running Applications
```bash
cargo run --bin eq_rng                     # Start web server
cargo run --bin eq_rng --features admin    # Start with admin interface
cargo run --bin test_db --package eq_rng_tests  # Run database tests
```

### 📋 Using Scripts (Recommended)
```bash
./run_tests.sh db            # Run database tests
./run_tests.sh build         # Test all packages build
./run_tests.sh all           # Run all test suites
```

### 🔨 Building
```bash
cargo build                           # Build entire workspace
cargo build --package eq_rng         # Build main app only
cargo build --package eq_rng_tests   # Build tests only
```

### 📊 Database Operations
```bash
sqlite3 data/zones.db "SELECT COUNT(*) FROM zones;"  # Check zone count
sqlite3 data/zones.db "SELECT name FROM zones LIMIT 5;"  # Sample zones

# Create new data.sql dump via admin interface
curl -X POST http://localhost:3000/admin/dump-database
```

## Admin Interface

The application includes an optional admin interface that can be enabled/disabled at build time using Cargo features:

### Admin Routes
When enabled, the admin interface provides:
- **Dashboard**: `/admin` - Overview of zones, instances, ratings, and statistics
- **Zone Management**: `/admin/zones` - Create, edit, delete, and move zones
- **Instance Management**: `/admin/instances` - Manage instances (zones moved from main zones)
- **Ratings Management**: `/admin/ratings` - View and manage zone ratings
- **Notes Management**: Add and manage categorized notes for zones and instances
- **Note Types**: `/admin/note-types` - Manage note categories (Epic 1.0, Epic 1.5, etc.)
- **Flag Types**: `/admin/flag-types` - Manage zone flag types and their appearance
- **Links Management**: `/admin/links` - Manage external links by category
- **Database Dump**: `🗄️ Dump Database to SQL` - Export database to timestamped SQL file

### Feature Control
- **Production builds**: Admin interface is **disabled** by default for security
- **Development builds**: Admin interface is **enabled** for testing and management
- **Local builds**: Admin interface can be enabled with `--features admin`

### Database Dump Feature
The admin interface includes a database dump feature:
1. Click "🗄️ Dump Database to SQL" button in admin dashboard
2. Creates `data/data-YYYYMMDD_HHMMSS.sql` with complete database export
3. File contains schema, data, indexes, and constraints
4. Can be used to replace `data/data.sql` for deployment

## Deployment

### Docker Deployment

#### Production Deployment (Recommended)
```bash
# Build and run production image (no admin interface)
./build.sh production
docker-compose up -d

# Or manually
docker-compose up -d --build
curl http://localhost:3000/random_zone
```

#### Development Deployment
```bash
# Build and run development image (with admin interface)
./build.sh development
docker-compose -f docker-compose.dev.yml up -d

# Access admin interface
curl http://localhost:3000/admin
```

### Fresh Environment Setup

For a completely fresh environment:

```bash
# Clone the repository
git clone <repository-url>
cd eq_rng

# Start the application (database auto-created from data.sql)
cargo run --bin eq_rng --features admin

# Verify database was created
sqlite3 data/zones.db "SELECT COUNT(*) FROM zones;"
```

### Data.sql Deployment Strategy

1. **Development**: Make changes via admin interface, dump to new data.sql
2. **Version Control**: Commit updated `data/data.sql` to repository
3. **Deployment**: Ship only `data.sql` file (not binary `zones.db`)
4. **Production**: Application creates database from `data.sql` on startup

## Build Configuration Summary

| Environment | Admin Interface | Build Command | Docker File |
|-------------|----------------|---------------|-------------|
| **Production** | ❌ Disabled | `./build.sh production` | `Dockerfile` |
| **Development** | ✅ Enabled | `./build.sh development` | `Dockerfile.dev` |
| **Local** | ✅ Enabled | `./build.sh local` | N/A |

## Database Management

- **Source**: `data/data.sql` (386 zones, all tables and data)
- **Generated**: `data/zones.db` (created automatically from data.sql)
- **Backups**: `data/data-YYYYMMDD_HHMMSS.sql` (admin dump feature)
- **Size**: ~330KB when populated
- **Reset**: Delete `data/zones.db*` files, restart application

## Migration from Old System

The previous JSON-based migration system has been completely replaced:

- ❌ **Removed**: `migrations/` directory and all JSON zone files
- ❌ **Removed**: Complex migration scripts and JSON parsing
- ✅ **New**: Single `data.sql` file as source of truth
- ✅ **New**: Timestamp-based loading system
- ✅ **New**: Admin dump functionality for backups

## Security Notes

- **Production deployments should always use builds without admin features**
- The admin interface has no authentication and should only be enabled in secure development environments
- Admin routes are completely excluded from the binary in production builds, reducing attack surface
- The `data.sql` file should be treated as source code and reviewed in PRs

## Rating Transaction Log System

The application includes a simple and secure file-based transaction log system for rating operations that enables seamless data preservation across deployments.

### Key Features

- **File-Based Logging**: All rating operations are logged as SQL statements to `data/rating_transaction.log`
- **Secure Design**: No API endpoints - transaction logs can only be accessed via file system
- **Simple Deployment**: Extract file from Docker, deploy new container, apply file to new container
- **Complete Audit Trail**: Every rating INSERT, UPDATE, DELETE operation is logged with timestamps
- **SQL Format**: Transaction logs are plain SQL files that can be easily inspected and applied

### Quick Start

```bash
# Extract transaction log before deployment
./utils/rating_transaction_log.sh extract

# Deploy new container
docker-compose up --build -d

# Apply transaction log to new container
./utils/rating_transaction_log.sh apply-to-docker backups/rating_transactions/rating_transactions_YYYYMMDD_HHMMSS.sql
```

### Deployment Workflow

1. **Pre-Deployment**: Extract `data/rating_transaction.log` from current container
2. **Deploy**: Build and start new container with updated code/database
3. **Post-Deployment**: Apply transaction log SQL file to preserve user rating data
4. **Verify**: Check that ratings are preserved in the new deployment

### Utilities

- `utils/rating_transaction_log.sh` - Extract, apply, and manage transaction log files
- `utils/deploy_with_rating_log.sh` - Complete deployment script with rating preservation
- `utils/test_rating_log.sh` - Test the transaction log functionality

### Security

- **No API endpoints** for transaction log management (secure by design)
- **File system access required** for all transaction log operations
- **SQL injection protection** with proper escaping of user input

For detailed documentation, see: `docs/RATING_TRANSACTION_LOG.md`

## Documentation

For detailed information about the data.sql migration system, see:
- `docs/data-sql-migration.md` - Complete migration system documentation
- `docs/RATING_TRANSACTION_LOG.md` - Simple file-based rating transaction log documentation
- Includes troubleshooting, examples, and technical details
