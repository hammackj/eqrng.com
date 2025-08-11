# EQRng.com

Server runs on port 3000 this is exported from docker to the host machine. The host machine is running ngix and proxies 3000 to 80/443. Currently the frontend is a dev test front end. I plan on making a proper frontend once I have all the features implemented.

## Project Structure

This is a Rust workspace with the following components:

### 📦 Packages

- **`eq_rng`** (`src/`) - Main web server and API endpoints
- **`eq_rng_migrations`** (`migrations/`) - Data migration utilities for seeding the database  
- **`eq_rng_tests`** (`tests/`) - Database testing and validation utilities

### 📁 Directory Layout

```
eq_rng.com/
├── src/                    # Main application source
│   ├── main.rs            # Web server entry point
│   ├── lib.rs             # Database setup and utilities
│   ├── zones.rs           # Zone API endpoints
│   ├── classes.rs         # Class API endpoints
│   ├── races.rs           # Race API endpoints
│   └── version.rs         # Version API endpoint
├── migrations/            # Migration subcrate
│   ├── src/
│   │   ├── lib.rs        # Migration library
│   │   └── migrate_zones.rs # Zone data migration
│   └── Cargo.toml        # Migration dependencies
├── tests/                 # Testing subcrate
│   ├── src/
│   │   ├── lib.rs        # Test utilities library
│   │   └── test_db.rs    # Database tests
│   └── Cargo.toml        # Test dependencies
├── data/                  # Data files and database
│   ├── zones.db          # SQLite database (ships with build)
│   ├── zones/            # JSON source files (40+ files)
│   └── class_race.json   # Class/race compatibility data
├── dist/                  # Frontend build output
├── run_migrations.sh      # Migration runner script
├── run_tests.sh          # Test runner script
└── Cargo.toml            # Workspace configuration
```

### 🔧 Available Binaries

- **`eq_rng`** - Main web server application
- **`migrate_zones`** - Database migration utility
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

### /random_class
#### Parameters
- race (optional)

Returns a random class, optionally filtered by race compatibility.

### /random_race

Returns a random race.

### /version

Returns the current application version.

## Data Management

### Database Location

The SQLite database is located at `data/zones.db` and can be shipped with the build. All data files are organized in the `data/` directory:

- `data/zones.db` - SQLite database with zone information
- `data/zones/` - JSON source files for zone data
- `data/class_race.json` - Class/race compatibility data

### Data Migration

The project includes a separate migrations subcrate for managing data imports:

#### Running Migrations

```bash
# Using the migration script (recommended)
./run_migrations.sh zones

# Or directly with cargo
cargo run --bin migrate_zones --package eq_rng_migrations
```

#### Migration Features

- Loads zone data from JSON files into SQLite database
- Creates database in `data/zones.db` for easy deployment
- Automatically detects if migration has already been run
- Supports running from project root or migrations directory
- All migrations are wrapped in database transactions

See `migrations/README.md` for more details.

## Testing

The project includes a separate tests subcrate for database validation and testing:

### Test Categories

- **Database Tests** - Connectivity, permissions, and data validation
- **Build Tests** - Verify all packages compile successfully
- **Application Tests** - Validate main database integrity

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
- Automatic cleanup of temporary test files

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

### Running migrations
```bash
./run_migrations.sh zones
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
cargo run --bin eq_rng                           # Start web server
cargo run --bin migrate_zones --package eq_rng_migrations  # Run migrations
cargo run --bin test_db --package eq_rng_tests   # Run database tests
```

### 📋 Using Scripts (Recommended)
```bash
./run_migrations.sh zones     # Run zone migration
./run_tests.sh db            # Run database tests
./run_tests.sh build         # Test all packages build
./run_tests.sh all           # Run all test suites
```

### 🔨 Building
```bash
cargo build                           # Build entire workspace
cargo build --package eq_rng         # Build main app only
cargo build --package eq_rng_migrations  # Build migrations only
cargo build --package eq_rng_tests   # Build tests only
```

### 📊 Database Operations
```bash
sqlite3 data/zones.db "SELECT COUNT(*) FROM zones;"  # Check zone count
sqlite3 data/zones.db "SELECT name FROM zones LIMIT 5;"  # Sample zones
```

# Deployment

## Admin Interface

The application includes an optional admin interface that can be enabled/disabled at build time using Cargo features:

### Admin Routes
When enabled, the admin interface provides:
- **Dashboard**: `/admin` - Overview of zones, ratings, and statistics
- **Zone Management**: `/admin/zones` - Create, edit, and delete zones
- **Ratings Management**: `/admin/ratings` - View and manage zone ratings
- **Notes Management**: `/admin/zones/:id/notes` - Add zone-specific notes
- **Note Types**: `/admin/note-types` - Manage note categories

### Feature Control
- **Production builds**: Admin interface is **disabled** by default for security
- **Development builds**: Admin interface is **enabled** for testing and management
- **Local builds**: Admin interface can be enabled with `--features admin`

## Database Shipping

The SQLite database (`data/zones.db`) is now included in the project and can be shipped with builds:

- **Docker builds**: The database is automatically included via the Dockerfile
- **Local builds**: The database is created in `data/zones.db` and can be committed to the repository
- **Fresh deployments**: If no database exists, run migrations to create and populate it

## Docker Deployment

### Production Deployment (Recommended)
```bash
# Build and run production image (no admin interface)
./build.sh production
docker-compose up -d

# Or manually
docker-compose up -d --build
curl http://localhost:3000/random_zone
```

### Development Deployment
```bash
# Build and run development image (with admin interface)
./build.sh development
docker-compose -f docker-compose.dev.yml up -d

# Access admin interface
curl http://localhost:3000/admin
```

## Fresh Environment Setup

For a completely fresh environment without a database:

```bash
# Clone the repository
git clone <repository-url>
cd eq_rng

# Run migrations to create and populate the database
./run_migrations.sh zones

# Start the application (production mode)
./build.sh local

# Or with admin interface for development
cargo run --bin eq_rng --features admin
```

## Build Configuration Summary

| Environment | Admin Interface | Build Command | Docker File |
|-------------|----------------|---------------|-------------|
| **Production** | ❌ Disabled | `./build.sh production` | `Dockerfile` |
| **Development** | ✅ Enabled | `./build.sh development` | `Dockerfile.dev` |
| **Local** | ✅ Enabled | `./build.sh local` | N/A |

## Database Management

- **Location**: `data/zones.db`
- **Size**: ~180KB when populated with 703 zones
- **Backup**: Simply copy the `data/zones.db` file
- **Reset**: Delete `data/zones.db*` files and re-run migrations

## Security Notes

- **Production deployments should always use builds without admin features**
- The admin interface has no authentication and should only be enabled in secure development environments
- Admin routes are completely excluded from the binary in production builds, reducing attack surface
