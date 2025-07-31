#!/bin/bash

# Auto-generated Zone Ratings Migration Script
# This script can be run during Docker rebuilds to reapply zone ratings

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKUP_FILE="$(dirname "$SCRIPT_DIR")/backups/zone_ratings/EOF
echo "zone_ratings_backup_${DATE}.${OUTPUT_FORMAT}\"" >> "$migration_script"

cat << 'EOF' >> "$migration_script"
DB_PATH="${DB_PATH:-data/zones.db}"
CONTAINER_NAME="${CONTAINER_NAME:-eq_rng-app-1}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[MIGRATION]${NC} $1"; }
log_success() { echo -e "${GREEN}[MIGRATION]${NC} $1"; }
log_error() { echo -e "${RED}[MIGRATION]${NC} $1"; }

# Check if running in Docker container
if [[ -f "/.dockerenv" ]] || [[ -n "$KUBERNETES_SERVICE_HOST" ]]; then
    DB_CMD="sqlite3 /opt/eq_rng/$DB_PATH"
else
    # Check if we should use Docker
    if docker ps --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$" 2>/dev/null; then
        DB_CMD="docker exec $CONTAINER_NAME sqlite3 /opt/eq_rng/$DB_PATH"
        log_info "Using Docker container: $CONTAINER_NAME"
    elif [[ -f "$DB_PATH" ]]; then
        DB_CMD="sqlite3 $DB_PATH"
        log_info "Using local database: $DB_PATH"
    else
        log_error "Cannot find database - neither Docker container nor local file available"
        exit 1
    fi
fi

# Wait for database to be available
log_info "Waiting for database to be available..."
for i in {1..30}; do
    if eval "$DB_CMD" <<< "SELECT COUNT(*) FROM zones;" >/dev/null 2>&1; then
        break
    fi
    if [[ $i -eq 30 ]]; then
        log_error "Database not available after 30 seconds"
        exit 1
    fi
    sleep 1
done

log_info "Database is available, checking if migration is needed..."

# Check if zones table exists and has data
ZONE_COUNT=$(eval "$DB_CMD" <<< "SELECT COUNT(*) FROM zones;" 2>/dev/null || echo "0")
if [[ "$ZONE_COUNT" -eq 0 ]]; then
    log_info "No zones found in database, skipping zone ratings migration"
    exit 0
fi

# Check if backup file exists
if [[ ! -f "$BACKUP_FILE" ]]; then
    log_error "Backup file not found: $BACKUP_FILE"
    exit 1
fi

log_info "Applying zone ratings from backup..."
log_info "Backup file: $BACKUP_FILE"
log_info "Total zones in database: $ZONE_COUNT"

# Apply the ratings based on backup format
BACKUP_FORMAT="${BACKUP_FILE##*.}"
case $BACKUP_FORMAT in
    "json")
        if command -v jq >/dev/null 2>&1; then
            TEMP_SQL="/tmp/zone_ratings_migration_$$.sql"
            jq -r '.[] | "UPDATE zones SET rating = \(.rating), verified = \(.verified) WHERE name = '\''\(.name)'\''; "' "$BACKUP_FILE" > "$TEMP_SQL"
            UPDATES=$(wc -l < "$TEMP_SQL")
            log_info "Applying $UPDATES zone rating updates..."
            eval "$DB_CMD" < "$TEMP_SQL"
            rm -f "$TEMP_SQL"
        else
            log_error "jq is required to apply JSON backup files"
            exit 1
        fi
        ;;
    "sql")
        log_info "Applying SQL zone rating updates..."
        eval "$DB_CMD" < "$BACKUP_FILE"
        ;;
    *)
        log_error "Unsupported backup format: $BACKUP_FORMAT"
        exit 1
        ;;
esac

# Verify the migration
RATED_ZONES=$(eval "$DB_CMD" <<< "SELECT COUNT(*) FROM zones WHERE rating > 0;")
log_success "Zone ratings migration completed successfully!"
log_info "Zones with ratings: $RATED_ZONES"
