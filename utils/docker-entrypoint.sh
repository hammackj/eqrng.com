#!/bin/bash

# Docker Entrypoint Script for eq_rng.com
# This script handles initialization tasks including zone ratings migration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[ENTRYPOINT]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[ENTRYPOINT]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[ENTRYPOINT]${NC} $1"
}

log_error() {
    echo -e "${RED}[ENTRYPOINT]${NC} $1"
}

# Configuration
DB_PATH="/opt/eq_rng/data/zones.db"
BACKUP_DIR="/opt/eq_rng/backups/zone_ratings"
MIGRATIONS_DIR="/opt/eq_rng/migrations/zone_ratings"
APP_BINARY="/opt/eq_rng/eq_rng"

# Environment variables with defaults
SKIP_ZONE_MIGRATION="${SKIP_ZONE_MIGRATION:-false}"
ZONE_MIGRATION_TIMEOUT="${ZONE_MIGRATION_TIMEOUT:-60}"
RUST_LOG="${RUST_LOG:-info}"

log_info "Starting eq_rng.com Docker container..."
log_info "Rust log level: $RUST_LOG"

# Function to wait for database to be ready
wait_for_database() {
    log_info "Waiting for database to be ready..."

    local timeout=$ZONE_MIGRATION_TIMEOUT
    local counter=0

    while [[ $counter -lt $timeout ]]; do
        if [[ -f "$DB_PATH" ]] && sqlite3 "$DB_PATH" "SELECT 1;" >/dev/null 2>&1; then
            log_success "Database is ready"
            return 0
        fi

        counter=$((counter + 1))
        sleep 1

        if [[ $((counter % 10)) -eq 0 ]]; then
            log_info "Still waiting for database... ($counter/$timeout seconds)"
        fi
    done

    log_error "Database not ready after $timeout seconds"
    return 1
}

# Function to check if zones table exists and has data
check_zones_table() {
    local zone_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM zones;" 2>/dev/null || echo "0")
    echo "$zone_count"
}

# Function to find and run the latest zone migration script
run_zone_migration() {
    log_info "Checking for zone ratings migration..."

    # Skip if explicitly disabled
    if [[ "$SKIP_ZONE_MIGRATION" == "true" ]]; then
        log_warning "Zone migration skipped (SKIP_ZONE_MIGRATION=true)"
        return 0
    fi

    # Check if migrations directory exists
    if [[ ! -d "$MIGRATIONS_DIR" ]]; then
        log_info "No zone migrations directory found, skipping migration"
        return 0
    fi

    # Find the latest migration script
    local latest_migration=$(find "$MIGRATIONS_DIR" -name "auto_migrate_zone_ratings_*.sh" -type f | sort | tail -1)

    if [[ -z "$latest_migration" ]]; then
        log_info "No zone migration scripts found, skipping migration"
        return 0
    fi

    log_info "Found zone migration script: $(basename "$latest_migration")"

    # Make sure it's executable
    chmod +x "$latest_migration"

    # Run the migration script
    log_info "Running zone ratings migration..."
    if "$latest_migration"; then
        log_success "Zone migration completed successfully"
    else
        log_warning "Zone migration script exited with non-zero status (this may be normal)"
    fi
}

# Function to run initial zone data migration if needed
run_initial_zone_migration() {
    local zone_count=$(check_zones_table)

    if [[ "$zone_count" -eq 0 ]]; then
        log_info "No zones found in database, running initial zone migration..."

        # Check if the migration binary exists
        if [[ -f "/opt/eq_rng/migrate_zones" ]]; then
            log_info "Running zone data migration..."
            /opt/eq_rng/migrate_zones
        elif [[ -f "/opt/eq_rng/migrations/target/release/migrate_zones" ]]; then
            log_info "Running zone data migration from migrations directory..."
            /opt/eq_rng/migrations/target/release/migrate_zones
        else
            log_warning "Zone migration binary not found, zones may need to be loaded manually"
        fi
    else
        log_info "Found $zone_count zones in database, skipping initial migration"
    fi
}

# Function to backup current zone ratings before starting
backup_current_ratings() {
    local zone_count=$(check_zones_table)

    if [[ "$zone_count" -gt 0 ]]; then
        log_info "Creating backup of current zone ratings..."

        # Create backup directory if it doesn't exist
        mkdir -p "$BACKUP_DIR"

        # Create a simple backup of current ratings
        local backup_file="$BACKUP_DIR/zone_ratings_startup_$(date +%Y%m%d_%H%M%S).sql"

        sqlite3 "$DB_PATH" "SELECT 'UPDATE zones SET rating = ' || rating || ', verified = ' || verified || ' WHERE name = ''' || name || ''';' FROM zones WHERE rating > 0 ORDER BY name;" > "$backup_file" 2>/dev/null || true

        if [[ -f "$backup_file" && -s "$backup_file" ]]; then
            log_success "Zone ratings backed up to: $(basename "$backup_file")"
        else
            log_info "No zone ratings to backup or backup failed"
            rm -f "$backup_file" 2>/dev/null || true
        fi
    fi
}

# Function to display startup information
show_startup_info() {
    log_info "Container startup information:"
    echo "================================"
    echo "Database path: $DB_PATH"
    echo "Database exists: $([ -f "$DB_PATH" ] && echo "Yes" || echo "No")"

    if [[ -f "$DB_PATH" ]]; then
        local zone_count=$(check_zones_table)
        local rated_zones=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM zones WHERE rating > 0;" 2>/dev/null || echo "0")
        echo "Total zones: $zone_count"
        echo "Zones with ratings: $rated_zones"
    fi

    echo "Skip zone migration: $SKIP_ZONE_MIGRATION"
    echo "Migration timeout: $ZONE_MIGRATION_TIMEOUT seconds"
    echo "================================"
}

# Function to handle shutdown signals
cleanup() {
    log_info "Received shutdown signal, cleaning up..."

    # Kill the application if it's running
    if [[ -n "$APP_PID" ]]; then
        log_info "Stopping application (PID: $APP_PID)..."
        kill -TERM "$APP_PID" 2>/dev/null || true
        wait "$APP_PID" 2>/dev/null || true
    fi

    log_success "Cleanup completed"
    exit 0
}

# Set up signal handlers
trap cleanup SIGTERM SIGINT

# Main startup sequence
main() {
    log_info "========================================="
    log_info "eq_rng.com Container Startup"
    log_info "========================================="

    # Show startup information
    show_startup_info

    # Wait for database to be available
    if ! wait_for_database; then
        log_error "Database initialization failed"
        exit 1
    fi

    # Backup current ratings if they exist
    backup_current_ratings

    # Run initial zone migration if needed
    run_initial_zone_migration

    # Run zone ratings migration
    run_zone_migration

    # Display final status
    local zone_count=$(check_zones_table)
    local rated_zones=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM zones WHERE rating > 0;" 2>/dev/null || echo "0")

    log_success "Startup sequence completed!"
    log_info "Final status: $zone_count total zones, $rated_zones with ratings"

    # Start the main application
    log_info "Starting eq_rng application..."
    log_info "Command: $APP_BINARY $@"

    # Execute the main application and capture its PID
    exec "$APP_BINARY" "$@" &
    APP_PID=$!

    # Wait for the application to finish
    wait "$APP_PID"
}

# Handle the case where the script is run directly vs being sourced
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
