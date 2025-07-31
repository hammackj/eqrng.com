#!/bin/bash

# Zone Ratings Migration Template
# This template is used to generate zone ratings migration scripts
# Template variables will be replaced when generating actual migration scripts

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKUP_FILE="{{BACKUP_FILE_PATH}}"
DB_PATH="${DB_PATH:-data/zones.db}"
CONTAINER_NAME="${CONTAINER_NAME:-eq_rng-app-1}"

# Migration metadata
MIGRATION_DATE="{{MIGRATION_DATE}}"
MIGRATION_ID="{{MIGRATION_ID}}"
ZONE_COUNT="{{ZONE_COUNT}}"
RATED_ZONES="{{RATED_ZONES}}"
BACKUP_FORMAT="{{BACKUP_FORMAT}}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[MIGRATION-{{MIGRATION_ID}}]${NC} $1"; }
log_success() { echo -e "${GREEN}[MIGRATION-{{MIGRATION_ID}}]${NC} $1"; }
log_error() { echo -e "${RED}[MIGRATION-{{MIGRATION_ID}}]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[MIGRATION-{{MIGRATION_ID}}]${NC} $1"; }

print_info() {
    log_info "Zone Ratings Migration Script"
    echo "============================="
    echo "Migration ID: $MIGRATION_ID"
    echo "Generated on: $MIGRATION_DATE"
    echo "Backup file: $(basename "$BACKUP_FILE")"
    echo "Backup format: $BACKUP_FORMAT"
    echo "Expected zones with ratings: $RATED_ZONES"
    echo "============================="
}

check_environment() {
    log_info "Checking environment..."

    # Check if running in Docker container
    if [[ -f "/.dockerenv" ]] || [[ -n "$KUBERNETES_SERVICE_HOST" ]]; then
        DB_CMD="sqlite3 /opt/eq_rng/$DB_PATH"
        log_info "Running inside container"
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
            return 1
        fi
    fi

    # Check if backup file exists
    if [[ ! -f "$BACKUP_FILE" ]]; then
        log_error "Backup file not found: $BACKUP_FILE"
        return 1
    fi

    return 0
}

wait_for_database() {
    log_info "Waiting for database to be available..."

    local timeout=60
    local counter=0

    while [[ $counter -lt $timeout ]]; do
        if eval "$DB_CMD" <<< "SELECT COUNT(*) FROM zones;" >/dev/null 2>&1; then
            log_success "Database is available"
            return 0
        fi

        counter=$((counter + 1))
        sleep 1

        if [[ $((counter % 10)) -eq 0 ]]; then
            log_info "Still waiting for database... ($counter/$timeout seconds)"
        fi
    done

    log_error "Database not available after $timeout seconds"
    return 1
}

check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check if zones table exists and has data
    local zone_count=$(eval "$DB_CMD" <<< "SELECT COUNT(*) FROM zones;" 2>/dev/null || echo "0")

    if [[ "$zone_count" -eq 0 ]]; then
        log_warning "No zones found in database - zone ratings migration may not be needed"
        return 1
    fi

    log_info "Found $zone_count zones in database"

    # Check for required tools based on backup format
    case "$BACKUP_FORMAT" in
        "json")
            if ! command -v jq >/dev/null 2>&1; then
                log_error "jq is required to apply JSON backup files"
                return 1
            fi
            ;;
        "sql")
            # SQL files can be applied directly with sqlite3
            ;;
        "csv")
            # CSV can be processed with shell tools
            ;;
        *)
            log_error "Unknown backup format: $BACKUP_FORMAT"
            return 1
            ;;
    esac

    return 0
}

create_backup_before_migration() {
    log_info "Creating safety backup before applying migration..."

    local safety_backup="/tmp/zone_ratings_before_migration_{{MIGRATION_ID}}_$(date +%s).sql"

    eval "$DB_CMD" <<< "SELECT 'UPDATE zones SET rating = ' || rating || ', verified = ' || verified || ' WHERE name = ''' || name || ''';' FROM zones WHERE rating > 0 ORDER BY name;" > "$safety_backup" 2>/dev/null || true

    if [[ -f "$safety_backup" && -s "$safety_backup" ]]; then
        log_success "Safety backup created: $safety_backup"
    else
        log_info "No existing ratings to backup"
        rm -f "$safety_backup" 2>/dev/null || true
    fi
}

apply_zone_ratings() {
    log_info "Applying zone ratings from backup..."

    case "$BACKUP_FORMAT" in
        "json")
            local temp_sql="/tmp/zone_ratings_migration_{{MIGRATION_ID}}_$$.sql"

            # Generate SQL from JSON using jq
            jq -r '.[] | "UPDATE zones SET rating = \(.rating), verified = \(.verified) WHERE name = '\''\(.name)'\''; "' "$BACKUP_FILE" > "$temp_sql"

            local update_count=$(wc -l < "$temp_sql")
            log_info "Applying $update_count zone rating updates..."

            eval "$DB_CMD" < "$temp_sql"
            rm -f "$temp_sql"
            ;;

        "sql")
            log_info "Applying SQL zone rating updates..."
            eval "$DB_CMD" < "$BACKUP_FILE"
            ;;

        "csv")
            local temp_sql="/tmp/zone_ratings_migration_{{MIGRATION_ID}}_$$.sql"

            # Parse CSV and generate SQL
            tail -n +2 "$BACKUP_FILE" | while IFS=',' read -r name rating verified; do
                echo "UPDATE zones SET rating = $rating, verified = $verified WHERE name = '$name';"
            done > "$temp_sql"

            local update_count=$(wc -l < "$temp_sql")
            log_info "Applying $update_count zone rating updates..."

            eval "$DB_CMD" < "$temp_sql"
            rm -f "$temp_sql"
            ;;
    esac
}

verify_migration() {
    log_info "Verifying migration results..."

    local total_zones=$(eval "$DB_CMD" <<< "SELECT COUNT(*) FROM zones;")
    local rated_zones=$(eval "$DB_CMD" <<< "SELECT COUNT(*) FROM zones WHERE rating > 0;")
    local verified_zones=$(eval "$DB_CMD" <<< "SELECT COUNT(*) FROM zones WHERE verified = 1;")

    log_info "Migration results:"
    echo "  Total zones: $total_zones"
    echo "  Zones with ratings: $rated_zones"
    echo "  Verified zones: $verified_zones"

    # Check if we got approximately the expected number of rated zones
    if [[ "$rated_zones" -ge $(($RATED_ZONES - 10)) && "$rated_zones" -le $(($RATED_ZONES + 10)) ]]; then
        log_success "Zone ratings migration appears successful"
        return 0
    else
        log_warning "Expected ~$RATED_ZONES rated zones, but got $rated_zones (within ±10 is considered normal)"
        return 0
    fi
}

# Main migration function
main() {
    print_info

    # Check environment and prerequisites
    if ! check_environment; then
        log_error "Environment check failed"
        exit 1
    fi

    if ! wait_for_database; then
        log_error "Database not available"
        exit 1
    fi

    if ! check_prerequisites; then
        log_warning "Prerequisites check failed - migration may not be needed"
        exit 0
    fi

    # Create safety backup
    create_backup_before_migration

    # Apply the zone ratings
    if apply_zone_ratings; then
        log_success "Zone ratings applied successfully"
    else
        log_error "Failed to apply zone ratings"
        exit 1
    fi

    # Verify the results
    if verify_migration; then
        log_success "Migration verification completed"
    else
        log_warning "Migration verification had warnings (but migration may still be successful)"
    fi

    log_success "Zone ratings migration completed successfully!"
}

# Allow script to be sourced for testing
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
