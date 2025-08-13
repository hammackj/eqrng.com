#!/usr/bin/env bash
#
# Restore zone_ratings SQL dump into a running Docker container's sqlite database.
# The script:
#  - Validates inputs and container state
#  - (Optional) makes a backup copy of the container DB before applying
#  - Copies the dump file into the container (default: /tmp/)
#  - Executes the dump inside the container using sqlite3
#  - Cleans up the temporary copy inside the container (unless asked to keep)
#
# Usage:
#   restore_zone_ratings_to_container.sh -i /path/to/dump.sql [options]
#
# Options:
#   -i, --input FILE           SQL dump file on the host (required)
#   -c, --container NAME       Docker container name (default: eq_rng-app-1)
#   -p, --db-path PATH         Path to sqlite DB inside the container (default: /opt/eq_rng/data/zones.db)
#       --skip-backup          Don't create a backup copy of the DB inside the container
#       --keep-temp            Keep the copied dump inside the container (default: removed)
#       --tmp-dir PATH         Directory inside container to copy dump to (default: /tmp)
#   -f, --force                Don't prompt for confirmation
#   -h, --help                 Show help
#
# Examples:
#   # Basic restore (will prompt)
#   ./restore_zone_ratings_to_container.sh -i backups/zone_ratings/zone_ratings_dump_20250101_120000.sql
#
#   # Restore to different container and DB path, skip backup, force
#   ./restore_zone_ratings_to_container.sh -i /tmp/zr.sql -c my-app -p /var/db/zones.sqlite --skip-backup --force
#

set -euo pipefail

# Defaults
CONTAINER_NAME="eqrng_app"
DB_PATH="/opt/eq_rng/data/zones.db"
INPUT_FILE=""
SKIP_BACKUP="false"
KEEP_TEMP="false"
TMP_DIR="/tmp"
FORCE="false"

usage() {
    sed -n '1,200p' "$0" | sed -n '1,140p'
    echo ""
    printf "Examples:\n  %s -i backups/zone_ratings/zone_ratings_dump.sql\n\n" "$0"
}

log()    { printf "\033[1;34m[INFO]\033[0m  %s\n" "$*"; }
log_ok() { printf "\033[1;32m[OK]\033[0m    %s\n" "$*"; }
log_warn(){ printf "\033[1;33m[WARN]\033[0m  %s\n" "$*"; }
log_err() { printf "\033[1;31m[ERR]\033[0m   %s\n" "$*"; }

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        -i|--input)
            INPUT_FILE="$2"
            shift 2
            ;;
        -c|--container)
            CONTAINER_NAME="$2"
            shift 2
            ;;
        -p|--db-path)
            DB_PATH="$2"
            shift 2
            ;;
        --skip-backup)
            SKIP_BACKUP="true"
            shift
            ;;
        --keep-temp)
            KEEP_TEMP="true"
            shift
            ;;
        --tmp-dir)
            TMP_DIR="$2"
            shift 2
            ;;
        -f|--force)
            FORCE="true"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            log_err "Unknown argument: $1"
            usage
            exit 2
            ;;
    esac
done

# Validate required input
if [[ -z "${INPUT_FILE:-}" ]]; then
    log_err "Input SQL dump file is required (-i/--input)"
    usage
    exit 2
fi

if [[ ! -f "$INPUT_FILE" ]]; then
    log_err "Input file not found: $INPUT_FILE"
    exit 3
fi

# Ensure docker CLI exists
if ! command -v docker >/dev/null 2>&1; then
    log_err "docker command not found in PATH"
    exit 4
fi

# Check container running
if ! docker ps --format '{{.Names}}' | grep -xq "$CONTAINER_NAME"; then
    log_err "Container '$CONTAINER_NAME' is not running"
    exit 5
fi

# Check sqlite3 exists inside container
if ! docker exec "$CONTAINER_NAME" sh -c "command -v sqlite3 >/dev/null 2>&1"; then
    log_err "sqlite3 not found inside container '$CONTAINER_NAME'. Aborting."
    exit 6
fi

# Confirm DB path exists inside the container (file may or may not exist; we will detect)
DB_EXISTS_IN_CONTAINER="false"
if docker exec "$CONTAINER_NAME" sh -c "[ -f '$DB_PATH' ] && echo yes || echo no" | tr -d '\r\n' | grep -q "^yes$"; then
    DB_EXISTS_IN_CONTAINER="true"
fi

# Summary and confirmation
log "Container: $CONTAINER_NAME"
log "DB path (container): $DB_PATH"
log "Input (host): $INPUT_FILE"
log "Temporary dir in container: $TMP_DIR"
log "Skip backup: $SKIP_BACKUP"
log "Keep temp in container after restore: $KEEP_TEMP"

if [[ "$FORCE" != "true" ]]; then
    echo ""
    echo -n "Proceed with restoring the dump into container '$CONTAINER_NAME'? (yes/no) > "
    read -r CONFIRM
    case "$CONFIRM" in
        yes|y|Y) ;;
        *) log_warn "Aborted by user"; exit 0 ;;
    esac
fi

TIMESTAMP="$(date -u +"%Y%m%d_%H%M%S")"
REMOTE_TMP_PATH="${TMP_DIR%/}/$(basename "$INPUT_FILE")"

# Copy file into container
log "Copying dump into container: $REMOTE_TMP_PATH"
if ! docker cp "$INPUT_FILE" "$CONTAINER_NAME:$REMOTE_TMP_PATH"; then
    log_err "Failed to copy dump into container"
    exit 7
fi

# Create backups directory inside container if needed
if [[ "$SKIP_BACKUP" != "true" ]]; then
    BACKUP_DIR="$(dirname "$DB_PATH")/../backups/database"
    # normalize path (simple)
    BACKUP_DIR="$(docker exec "$CONTAINER_NAME" sh -c "mkdir -p '$BACKUP_DIR' && echo \"$BACKUP_DIR\"")" || BACKUP_DIR=""
fi

# Backup DB inside container
if [[ "$SKIP_BACKUP" != "true" ]]; then
    if [[ "$DB_EXISTS_IN_CONTAINER" == "true" ]]; then
        REMOTE_BACKUP_PATH="${BACKUP_DIR%/}/zones.db.bak_${TIMESTAMP}"
        log "Creating backup of DB inside container: $REMOTE_BACKUP_PATH"
        if ! docker exec "$CONTAINER_NAME" sh -c "cp '$DB_PATH' '$REMOTE_BACKUP_PATH'"; then
            log_err "Failed to create DB backup inside container"
            # Offer to continue or abort
            if [[ "$FORCE" != "true" ]]; then
                echo -n "Backup failed. Continue without backup? (yes/no) > "
                read -r cont
                case "$cont" in
                    yes|y|Y) log_warn "Continuing without backup";;
                    *) log_warn "Aborting"; docker exec "$CONTAINER_NAME" sh -c "rm -f '$REMOTE_TMP_PATH' >/dev/null 2>&1 || true"; exit 8;;
                esac
            else
                log_warn "Forced: continuing without backup"
            fi
        else
            log_ok "Backup created: $REMOTE_BACKUP_PATH"
        fi
    else
        log_warn "DB file does not exist in container; skipping backup (DB will be created when sqlite3 runs)"
    fi
else
    log_warn "Skipping backup as requested"
fi

# Run the SQL inside the container using sqlite3 in a subshell so that redirection occurs inside container
log "Applying SQL dump inside the container..."
# We will run: sh -c "sqlite3 'DB_PATH' < 'REMOTE_TMP_PATH'"
APPLY_CMD="sqlite3 '$DB_PATH' < '$REMOTE_TMP_PATH'"

if ! docker exec "$CONTAINER_NAME" sh -c "$APPLY_CMD"; then
    log_err "Failed to apply SQL dump inside the container"
    log_warn "If a backup was created you can restore it with: docker exec $CONTAINER_NAME sh -c \"cp '$REMOTE_BACKUP_PATH' '$DB_PATH'\""
    # attempt cleanup of temp file if we created it and not asked to keep
    if [[ "$KEEP_TEMP" != "true" ]]; then
        docker exec "$CONTAINER_NAME" sh -c "rm -f '$REMOTE_TMP_PATH' >/dev/null 2>&1 || true" || true
    fi
    exit 9
fi

log_ok "SQL dump applied successfully"

# Optionally remove the temporary file in container
if [[ "$KEEP_TEMP" == "true" ]]; then
    log "Keeping temporary dump inside container: $REMOTE_TMP_PATH"
else
    log "Removing temporary dump from container"
    docker exec "$CONTAINER_NAME" sh -c "rm -f '$REMOTE_TMP_PATH' || true" || true
fi

log_ok "Restore complete"

# Show simple verification of rows count (if table exists)
log "Attempting to show number of zone_ratings rows..."
# Use a safe query that will not fail the script if table missing
docker exec "$CONTAINER_NAME" sh -c "sqlite3 '$DB_PATH' \"SELECT 'zone_ratings rows: ' || COUNT(*) FROM zone_ratings;\" 2>/dev/null" || true

exit 0
