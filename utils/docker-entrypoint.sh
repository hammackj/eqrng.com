#!/bin/bash
#
# Docker Entrypoint Script for eq_rng.com
# - Initializes DB-related tasks (backups, migrations)
# - Optionally applies rating transaction logs in background
# - Starts the eq_rng binary and handles graceful shutdown
#
# This file replaces the previously-broken entrypoint. It defines missing
# helpers and fixes a malformed background job block.
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[ENTRYPOINT]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[ENTRYPOINT]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[ENTRYPOINT]${NC} $*"
}

log_error() {
    echo -e "${RED}[ENTRYPOINT]${NC} $*"
}

# Configuration (can be overridden via env)
DB_PATH="${DB_PATH:-/opt/eq_rng/data/zones.db}"
BACKUP_DIR="${BACKUP_DIR:-/opt/eq_rng/backups/zone_ratings}"
MIGRATIONS_DIR="${MIGRATIONS_DIR:-/opt/eq_rng/migrations/zone_ratings}"
APP_BINARY="${APP_BINARY:-/opt/eq_rng/eq_rng}"

# Script that applies rating transaction logs (optional). Can be set via env.
AUTO_APPLY_SCRIPT="${AUTO_APPLY_SCRIPT:-/opt/eq_rng/utils/apply_rating_transactions.sh}"

# Environment flags with defaults
SKIP_ZONE_MIGRATION="${SKIP_ZONE_MIGRATION:-false}"
SKIP_TRANSACTION_AUTO_APPLY="${SKIP_TRANSACTION_AUTO_APPLY:-false}"
SKIP_DATABASE_WAIT="${SKIP_DATABASE_WAIT:-false}"
ZONE_MIGRATION_TIMEOUT="${ZONE_MIGRATION_TIMEOUT:-60}"
RUST_LOG="${RUST_LOG:-info}"
DEBUG_ENTRYPOINT="${DEBUG_ENTRYPOINT:-false}"

# Show helpful startup info (used early in main)
show_startup_info() {
    log_info "Container startup information:"
    echo "================================"
    echo "Database path: $DB_PATH"
    echo "Database exists: $([ -f "$DB_PATH" ] && echo "Yes" || echo "No")"
    if [[ "$DEBUG_ENTRYPOINT" == "true" ]]; then
        echo "Database directory contents:"
        ls -la "$(dirname "$DB_PATH")" 2>/dev/null || echo "  Cannot list directory"
        echo "Database file permissions:"
        ls -la "$DB_PATH" 2>/dev/null || echo "  Cannot stat file"
    fi
    if [[ -f "$DB_PATH" ]]; then
        if command -v sqlite3 >/dev/null 2>&1; then
            local zone_count
            zone_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM zones;" 2>/dev/null || echo "0")
            local rated_zones
            rated_zones=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM zones WHERE rating > 0;" 2>/dev/null || echo "0")
            echo "Total zones: $zone_count"
            echo "Zones with ratings: $rated_zones"
        else
            echo "sqlite3 not available in the image; cannot inspect DB contents"
        fi
    fi
    echo "Skip zone migration: $SKIP_ZONE_MIGRATION"
    echo "Skip transaction auto-apply: $SKIP_TRANSACTION_AUTO_APPLY"
    echo "Migration timeout: $ZONE_MIGRATION_TIMEOUT seconds"
    echo "================================"
}

# Wait for the database file to be ready and responsive (only if it should already exist)
wait_for_database() {
    # If database doesn't exist and the app is supposed to create it, skip waiting
    if [[ ! -f "$DB_PATH" && -f "/opt/eq_rng/data/data.sql" ]]; then
        log_info "Database will be created by application from data.sql, skipping wait"
        return 0
    fi

    # If database doesn't exist and no data.sql, skip waiting (app will create empty DB)
    if [[ ! -f "$DB_PATH" ]]; then
        log_info "Database will be created by application, skipping wait"
        return 0
    fi

    log_info "Database exists, verifying it's accessible..."

    local timeout=${ZONE_MIGRATION_TIMEOUT:-60}
    local counter=0

    while (( counter < timeout )); do
        if [[ -f "$DB_PATH" ]]; then
            if command -v sqlite3 >/dev/null 2>&1 && sqlite3 "$DB_PATH" "SELECT 1;" >/dev/null 2>&1; then
                log_success "Database is ready"
                return 0
            fi
        fi

        counter=$((counter + 1))
        sleep 1

        if (( counter % 10 == 0 )); then
            log_info "Still waiting for database... ($counter/$timeout seconds)"
        fi
    done

    log_error "Database not ready after $timeout seconds"
    return 1
}

# Returns number of zones (0 if DB missing or error)
check_zones_table() {
    if [[ -f "$DB_PATH" ]] && command -v sqlite3 >/dev/null 2>&1; then
        sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM zones;" 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

# Run the latest zone ratings migration script found in MIGRATIONS_DIR
run_zone_migration() {
    log_info "Checking for zone ratings migration..."

    if [[ "$SKIP_ZONE_MIGRATION" == "true" ]]; then
        log_warning "Zone migration skipped (SKIP_ZONE_MIGRATION=true)"
        return 0
    fi

    if [[ ! -d "$MIGRATIONS_DIR" ]]; then
        log_info "No zone migrations directory found at $MIGRATIONS_DIR, skipping migration"
        return 0
    fi

    local latest_migration
    # find the newest file matching pattern
    latest_migration=$(find "$MIGRATIONS_DIR" -maxdepth 1 -type f -name "auto_migrate_zone_ratings_*.sh" -print0 2>/dev/null | xargs -0 -n1 | sort || true)
    latest_migration=$(echo "$latest_migration" | tail -1)

    if [[ -z "$latest_migration" ]]; then
        log_info "No zone migration scripts found, skipping migration"
        return 0
    fi

    log_info "Found zone migration script: $(basename "$latest_migration")"
    chmod +x "$latest_migration"

    log_info "Running zone ratings migration..."
    if "$latest_migration"; then
        log_success "Zone migration completed successfully"
    else
        log_warning "Zone migration script exited with non-zero status (this may be normal)"
    fi
}

# Run initial zone data migration if zones table is empty
run_initial_zone_migration() {
    local zone_count
    zone_count=$(check_zones_table)

    if [[ "$zone_count" -eq 0 ]]; then
        log_info "No zones found in database, running initial zone migration..."

        if [[ -x "/opt/eq_rng/migrate_zones" ]]; then
            log_info "Running zone data migration binary at /opt/eq_rng/migrate_zones..."
            /opt/eq_rng/migrate_zones || log_warning "migrate_zones returned non-zero"
        elif [[ -x "/opt/eq_rng/migrations/target/release/migrate_zones" ]]; then
            log_info "Running zone data migration binary at migrations/target/release..."
            /opt/eq_rng/migrations/target/release/migrate_zones || log_warning "migrate_zones returned non-zero"
        else
            log_warning "Zone migration binary not found; zones may need to be loaded manually"
        fi
    else
        log_info "Found $zone_count zones in database, skipping initial migration"
    fi
}

# Backup current zone ratings into a SQL file (simple export)
backup_current_ratings() {
    local zone_count
    zone_count=$(check_zones_table)

    if [[ "${zone_count:-0}" -gt 0 ]]; then
        log_info "Creating backup of current zone ratings..."

        mkdir -p "$BACKUP_DIR"

        local backup_file="$BACKUP_DIR/zone_ratings_startup_$(date +%Y%m%d_%H%M%S).sql"

        # Export simple UPDATE statements for zones with rating > 0
        if command -v sqlite3 >/dev/null 2>&1; then
            sqlite3 "$DB_PATH" "SELECT 'UPDATE zones SET rating = ' || rating || ', verified = ' || verified || ' WHERE name = ''' || replace(name, '''', '''''') || ''';' FROM zones WHERE rating > 0 ORDER BY name;" > "$backup_file" 2>/dev/null || true
        fi

        if [[ -f "$backup_file" && -s "$backup_file" ]]; then
            log_success "Zone ratings backed up to: $(basename "$backup_file")"
        else
            log_info "No zone ratings to backup or backup failed"
            rm -f "$backup_file" 2>/dev/null || true
        fi
    else
        log_info "No zones present; skipping ratings backup"
    fi
}

# Auto-apply rating transaction logs in background (non-blocking)
auto_apply_transactions() {
    log_info "Checking for rating transaction logs to apply..."

    if [[ "$SKIP_TRANSACTION_AUTO_APPLY" == "true" ]]; then
        log_warning "Transaction log auto-apply skipped (SKIP_TRANSACTION_AUTO_APPLY=true)"
        return 0
    fi

    if [[ ! -f "$AUTO_APPLY_SCRIPT" ]]; then
        log_info "Transaction auto-apply script not found at $AUTO_APPLY_SCRIPT, skipping"
        return 0
    fi

    chmod +x "$AUTO_APPLY_SCRIPT"

    # Run the auto-apply script in background after a short delay to let the API come up
    local log_file="/opt/eq_rng/logs/tx_auto_apply_$(date +%Y%m%d_%H%M%S).log"
    mkdir -p "/opt/eq_rng/logs"

    log_info "Starting transaction log auto-apply in background; logs -> $log_file"
    (
        # give the main app a few seconds to start and accept connections
        sleep 5
        log_info "Auto-apply: running $AUTO_APPLY_SCRIPT"
        if "$AUTO_APPLY_SCRIPT" >> "$log_file" 2>&1; then
            log_success "Auto-apply completed, output saved to $log_file"
        else
            log_warning "Auto-apply script exited with non-zero, see $log_file"
        fi
    ) &
    # store PID of background job so cleanup can wait/kill if necessary
    AUTO_APPLY_PID=$!
    log_info "Auto-apply background PID: ${AUTO_APPLY_PID:-unknown}"
}

# Cleanup/shutdown handler
cleanup() {
    log_info "Received shutdown signal, cleaning up..."

    # kill auto-apply job if running
    if [[ -n "${AUTO_APPLY_PID:-}" ]]; then
        if kill -0 "$AUTO_APPLY_PID" >/dev/null 2>&1; then
            log_info "Stopping auto-apply job (PID: $AUTO_APPLY_PID)..."
            kill -TERM "$AUTO_APPLY_PID" 2>/dev/null || true
            wait "$AUTO_APPLY_PID" 2>/dev/null || true
        fi
    fi

    # Kill the application if it's running
    if [[ -n "${APP_PID:-}" ]]; then
        if kill -0 "$APP_PID" >/dev/null 2>&1; then
            log_info "Stopping application (PID: $APP_PID)..."
            kill -TERM "$APP_PID" 2>/dev/null || true
            wait "$APP_PID" 2>/dev/null || true
        fi
    fi

    log_success "Cleanup completed"
    exit 0
}

trap cleanup SIGTERM SIGINT

# Main startup sequence
main() {
    log_info "========================================="
    log_info "eq_rng.com Container Startup"
    log_info "========================================="

    show_startup_info

    # If the SQLite DB file is missing but a data SQL dump is present, create the DB from the SQL.
    # This allows first-time container startup to initialize the database automatically.
    if [[ ! -f "$DB_PATH" && -f "/opt/eq_rng/data/data.sql" ]]; then
        log_info "Database file $DB_PATH not found but data/data.sql exists; creating database from SQL..."
        mkdir -p "$(dirname "$DB_PATH")"
        if command -v sqlite3 >/dev/null 2>&1; then
            if sqlite3 "$DB_PATH" < /opt/eq_rng/data/data.sql 2>/opt/eq_rng/logs/db_init_error.log; then
                log_success "Created $DB_PATH from data/data.sql"
            else
                log_error "Failed to create $DB_PATH from data/data.sql (see /opt/eq_rng/logs/db_init_error.log)"
            fi
        else
            log_error "sqlite3 not available in the image; cannot create database from data/data.sql"
        fi
    fi

    # Wait for database to be available (only if it already exists)
    if ! wait_for_database; then
        log_error "Database initialization failed"
        exit 1
    fi

    backup_current_ratings
    run_initial_zone_migration
    run_zone_migration

    # Start auto-apply in background (non-blocking)
    auto_apply_transactions

    # Final status
    local zone_count
    zone_count=$(check_zones_table)
    local rated_zones=0
    if [[ -f "$DB_PATH" && $(command -v sqlite3 >/dev/null 2>&1; echo $?) -eq 0 ]]; then
        rated_zones=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM zones WHERE rating > 0;" 2>/dev/null || echo "0")
    fi

    log_success "Startup sequence completed!"
    log_info "Final status: $zone_count total zones, $rated_zones with ratings"

    # Start the main application in background so we can trap and cleanup
    if [[ ! -x "$APP_BINARY" ]]; then
        log_error "Application binary not found or not executable: $APP_BINARY"
        exit 2
    fi

    # If Docker provided the same executable name as a CMD (common when image CMD is ["./eq_rng"]),
    # drop that redundant single argument so the application doesn't receive it.
    if [[ $# -eq 1 && ( "$1" == "./$(basename "$APP_BINARY")" || "$1" == "$(basename "$APP_BINARY")" ) ]]; then
        set --
    fi

    log_info "Starting eq_rng application..."
    if [[ $# -gt 0 ]]; then
        log_info "Command: $APP_BINARY $*"
        # Start application with provided args
        "$APP_BINARY" "$@" &
    else
        log_info "Command: $APP_BINARY"
        # Start application with no extra args
        "$APP_BINARY" &
    fi
    APP_PID=$!

    # Wait for the application process; this allows trap to work
    wait "$APP_PID"
    local rc=$?

    log_info "Application exited with code: $rc"

    # Ensure auto-apply background job is terminated if still running
    if [[ -n "${AUTO_APPLY_PID:-}" ]]; then
        if kill -0 "$AUTO_APPLY_PID" >/dev/null 2>&1; then
            kill -TERM "$AUTO_APPLY_PID" 2>/dev/null || true
            wait "$AUTO_APPLY_PID" 2>/dev/null || true
        fi
    fi

    return $rc
}

# If the script is executed (not sourced), run main
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    log_info "Starting eq_rng.com Docker container..."
    log_info "Rust log level: $RUST_LOG"
    main "$@"
fi
