#!/usr/bin/env bash
#
# Dump zone_ratings from a Docker container (or local sqlite file) into a SQL file
# suitable for re-import (creates DELETE + INSERT statements and restores sqlite_sequence).
#
# Usage:
#   dump_zone_ratings_from_docker.sh [options]
#
# Options:
#   -d, --docker            Use Docker container (default behavior)
#   -l, --local             Use local sqlite DB file instead of Docker
#   -c, --container NAME    Docker container name (default: eq_rng-app-1)
#   -p, --db-path PATH      Path to sqlite DB relative to /opt/eq_rng in container or local path (default: data/zones.db)
#   -o, --out DIR           Output directory (default: backups/zone_ratings)
#   -f, --file NAME         Output filename (default: zone_ratings_dump_<timestamp>.sql)
#   --force                 Overwrite output file if it exists
#   -h, --help              Show this help
#
# Examples:
#   # Dump from running container (default)
#   ./dump_zone_ratings_from_docker.sh
#
#   # Dump from a specific container and output file
#   ./dump_zone_ratings_from_docker.sh -c my-container -o /tmp -f my_dump.sql
#
#   # Dump from local database file
#   ./dump_zone_ratings_from_docker.sh --local --db-path ./data/zones.db
#
#   # Feed file into sqlite inside container (will run the DELETE + INSERT + sqlite_sequence commands)
#   docker exec -i eq_rng-app-1 sqlite3 /opt/eq_rng/data/zones.db < backups/zone_ratings/zone_ratings_dump_YYYYMMDD_HHMMSS.sql
#
#   # Feed file into local sqlite DB file
#   sqlite3 data/zones.db < backups/zone_ratings/zone_ratings_dump_YYYYMMDD_HHMMSS.sql
#

set -euo pipefail

# Defaults
USE_DOCKER="true"
CONTAINER_NAME="eqrng_app"
DB_PATH="data/zones.db"
OUT_DIR="backups/zone_ratings"
TIMESTAMP="$(date +"%Y%m%d_%H%M%S")"
OUT_FILE=""
FORCE="false"

print_usage() {
    sed -n '1,200p' "$0" | sed -n '1,120p'
    echo ""
}

# Helper logging
log_info()    { printf "\033[1;34m[INFO]\033[0m  %s\n" "$*"; }
log_success() { printf "\033[1;32m[SUCCESS]\033[0m %s\n" "$*"; }
log_warn()    { printf "\033[1;33m[WARN]\033[0m    %s\n" "$*"; }
log_error()   { printf "\033[1;31m[ERROR]\033[0m   %s\n" "$*"; }

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        -d|--docker)
            USE_DOCKER="true"
            shift
            ;;
        -l|--local)
            USE_DOCKER="false"
            shift
            ;;
        -c|--container)
            CONTAINER_NAME="$2"
            shift 2
            ;;
        -p|--db-path)
            DB_PATH="$2"
            shift 2
            ;;
        -o|--out)
            OUT_DIR="$2"
            shift 2
            ;;
        -f|--file)
            OUT_FILE="$2"
            shift 2
            ;;
        --force)
            FORCE="true"
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            log_error "Unknown argument: $1"
            print_usage
            exit 2
            ;;
    esac
done

# Normalize OUT_FILE default
if [[ -z "$OUT_FILE" ]]; then
    OUT_FILE="zone_ratings_dump_${TIMESTAMP}.sql"
fi

mkdir -p "$OUT_DIR"
OUT_PATH="${OUT_DIR%/}/${OUT_FILE}"

if [[ -f "$OUT_PATH" ]] && [[ "$FORCE" != "true" ]]; then
    log_error "Output file already exists: $OUT_PATH"
    log_info "Use --force to overwrite"
    exit 1
fi

# Determine DB command
DB_CMD=""
if [[ "$USE_DOCKER" == "true" ]]; then
    # Check docker present
    if ! command -v docker >/dev/null 2>&1; then
        log_error "docker not found in PATH"
        exit 1
    fi

    # Check container running
    if ! docker ps --format '{{.Names}}' | grep -xq "$CONTAINER_NAME"; then
        log_error "Docker container '$CONTAINER_NAME' is not running"
        exit 1
    fi

    # Prefer sqlite3 inside container; verify it exists
    if ! docker exec "$CONTAINER_NAME" sh -c "command -v sqlite3 >/dev/null 2>&1"; then
        log_error "sqlite3 not found inside container '$CONTAINER_NAME'"
        exit 1
    fi

    # Build DB command string: it will read SQL from stdin
    DB_CMD="docker exec -i ${CONTAINER_NAME} sqlite3 /opt/eq_rng/${DB_PATH}"
else
    # local mode: need sqlite3 binary and local file
    if ! command -v sqlite3 >/dev/null 2>&1; then
        log_error "sqlite3 not found locally"
        exit 1
    fi
    if [[ ! -f "$DB_PATH" ]]; then
        log_error "Local database file not found: $DB_PATH"
        exit 1
    fi
    DB_CMD="sqlite3 ${DB_PATH}"
fi

log_info "Dumping zone_ratings from $( [[ "$USE_DOCKER" == "true" ]] && echo "Docker container '$CONTAINER_NAME' (DB: /opt/eq_rng/$DB_PATH)" || echo "local DB '$DB_PATH'")"
log_info "Output file: $OUT_PATH"

# SQL generation:
# 1) header and pragmas
# 2) BEGIN TRANSACTION;
# 3) DELETE FROM zone_ratings;
# 4) INSERT statements for all rows (use quote() to safely quote text)
# 5) Update sqlite_sequence for zone_ratings
# 6) COMMIT; PRAGMA foreign_keys=ON;

{
    printf "%s\n" "-- zone_ratings dump generated on $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    printf "%s\n" "-- Source: $( [[ "$USE_DOCKER" == "true" ]] && echo "docker:$CONTAINER_NAME:/opt/eq_rng/$DB_PATH" || echo "local:$DB_PATH" )"
    printf "%s\n" ""
    printf "%s\n" "PRAGMA foreign_keys=OFF;"
    printf "%s\n" "BEGIN TRANSACTION;"
    printf "%s\n" "DELETE FROM zone_ratings;"
    printf "%s\n" ""
} > "$OUT_PATH"

# Produce INSERT statements using sqlite's quote() helper to escape strings safely
GENERATE_INSERT_SQL=$(
cat <<'SQL'
SELECT 'INSERT INTO zone_ratings(id, zone_id, user_ip, rating, created_at, updated_at) VALUES(' ||
       id || ',' || zone_id || ',' || quote(user_ip) || ',' || rating || ',' ||
       quote(created_at) || ',' || quote(updated_at) || ');'
  FROM zone_ratings
 ORDER BY id;
SQL
)

# Run the SQL to produce the INSERT lines and append to file
if ! eval "$DB_CMD" <<< "$GENERATE_INSERT_SQL" >> "$OUT_PATH" 2>/dev/null; then
    log_error "Failed to read zone_ratings from database"
    # cleanup partial output unless force requested
    if [[ "$FORCE" != "true" ]] && [[ -f "$OUT_PATH" ]]; then
        rm -f "$OUT_PATH" || true
    fi
    exit 1
fi

# Compute max id for sqlite_sequence
GET_MAX_ID_SQL="SELECT COALESCE(MAX(id), 0) FROM zone_ratings;"
MAX_ID="$(eval "$DB_CMD" <<< "$GET_MAX_ID_SQL" | tr -d '\r\n' || echo "0")"
if [[ -z "$MAX_ID" ]]; then
    MAX_ID="0"
fi

{
    printf "%s\n" ""
    printf "%s\n" "-- Restore sqlite_sequence for zone_ratings (so AUTOINCREMENT picks up correctly)"
    # Use INSERT OR REPLACE to set sqlite_sequence
    printf "INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES('zone_ratings', %s);\n" "$MAX_ID"
    printf "%s\n" "COMMIT;"
    printf "%s\n" "PRAGMA foreign_keys=ON;"
} >> "$OUT_PATH"

log_success "Zone_ratings successfully dumped to: $OUT_PATH"
log_info "Rows exported: $(grep -c '^INSERT INTO zone_ratings' "$OUT_PATH" || true)"
exit 0
