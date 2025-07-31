#!/bin/bash

# Zone Ratings Backup and Migration Utility for eq_rng.com
# This script backs up zone ratings from the database and creates migration scripts
# that can be reapplied during Docker rebuilds or database updates

set -e  # Exit on any error

# Configuration
DB_PATH="${DB_PATH:-data/zones.db}"
BACKUP_DIR="${BACKUP_DIR:-backups/zone_ratings}"
MIGRATIONS_DIR="${MIGRATIONS_DIR:-migrations/zone_ratings}"
DATE=$(date +"%Y%m%d_%H%M%S")
CONTAINER_NAME="${CONTAINER_NAME:-eq_rng-app-1}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_usage() {
    echo "Zone Ratings Backup and Migration Utility"
    echo "========================================="
    echo ""
    echo "Usage: $0 [OPTIONS] [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  backup              Create backup of zone ratings (default)"
    echo "  migrate             Apply zone ratings from backup/migration file"
    echo "  auto-migrate        Create backup and generate auto-migration script"
    echo "  list-backups        List available zone rating backups"
    echo "  status              Show current zone ratings status"
    echo ""
    echo "Options:"
    echo "  -h, --help          Show this help message"
    echo "  -d, --docker        Use Docker container for database operations"
    echo "  -l, --local         Use local filesystem for database operations (default)"
    echo "  -c, --container NAME Docker container name (default: eq_rng-app-1)"
    echo "  -o, --output DIR    Specify backup directory (default: backups/zone_ratings)"
    echo "  -f, --file FILE     Specify backup/migration file to use"
    echo "  --force             Force operations without confirmation"
    echo "  --format FORMAT     Output format: json, sql, csv (default: json)"
    echo "  --min-rating NUM    Only backup zones with rating >= NUM (default: 1)"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Create local backup of zone ratings"
    echo "  $0 backup --docker                   # Create backup from Docker container"
    echo "  $0 migrate -f backup_20231215.json   # Apply ratings from specific backup"
    echo "  $0 auto-migrate                      # Create backup and auto-migration script"
    echo "  $0 status --docker                   # Show current ratings status in Docker"
    echo ""
    echo "Auto-Migration:"
    echo "  The auto-migrate command creates a migration script that can be run during"
    echo "  Docker rebuilds to automatically reapply zone ratings. Add the generated"
    echo "  script to your Docker build process or startup scripts."
}

check_dependencies() {
    if [[ "$USE_DOCKER" == "true" ]]; then
        if ! command -v docker &> /dev/null; then
            log_error "Docker is not installed or not in PATH"
            exit 1
        fi
    fi

    if ! command -v sqlite3 &> /dev/null; then
        log_warning "sqlite3 not found - some operations may be limited"
    fi

    if ! command -v jq &> /dev/null && [[ "$OUTPUT_FORMAT" == "json" ]]; then
        log_warning "jq not found - JSON output will be basic"
    fi
}

create_directories() {
    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_info "Creating backup directory: $BACKUP_DIR"
        mkdir -p "$BACKUP_DIR"
    fi

    if [[ ! -d "$MIGRATIONS_DIR" ]]; then
        log_info "Creating migrations directory: $MIGRATIONS_DIR"
        mkdir -p "$MIGRATIONS_DIR"
    fi
}

get_db_connection() {
    if [[ "$USE_DOCKER" == "true" ]]; then
        # Check if container is running
        if ! docker ps --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
            log_error "Docker container '$CONTAINER_NAME' is not running"
            exit 1
        fi
        echo "docker exec $CONTAINER_NAME sqlite3 /opt/eq_rng/$DB_PATH"
    else
        if [[ ! -f "$DB_PATH" ]]; then
            log_error "Database file not found: $DB_PATH"
            exit 1
        fi
        echo "sqlite3 $DB_PATH"
    fi
}

export_zone_ratings() {
    local output_file="$1"
    local format="$2"
    local min_rating="$3"

    log_info "Exporting zone ratings (format: $format, min_rating: $min_rating)..."

    local db_cmd=$(get_db_connection)
    local sql_query="SELECT name, rating, verified FROM zones WHERE rating >= $min_rating ORDER BY name;"

    case $format in
        "json")
            # Create JSON format
            local temp_file="/tmp/zone_ratings_$$.tmp"
            eval "$db_cmd" <<< "$sql_query" > "$temp_file"

            echo "[" > "$output_file"
            local first_line=true
            while IFS='|' read -r name rating verified; do
                if [[ "$first_line" == "true" ]]; then
                    first_line=false
                else
                    echo "," >> "$output_file"
                fi
                cat << EOF >> "$output_file"
  {
    "name": "$name",
    "rating": $rating,
    "verified": $([ "$verified" == "1" ] && echo "true" || echo "false")
  }
EOF
            done < "$temp_file"
            echo "" >> "$output_file"
            echo "]" >> "$output_file"
            rm -f "$temp_file"
            ;;

        "sql")
            # Create SQL format
            echo "-- Zone Ratings Backup - Generated on $(date)" > "$output_file"
            echo "-- Minimum rating: $min_rating" >> "$output_file"
            echo "" >> "$output_file"
            echo "-- Update zone ratings" >> "$output_file"
            eval "$db_cmd" <<< "SELECT 'UPDATE zones SET rating = ' || rating || ', verified = ' || verified || ' WHERE name = ''' || name || ''';' FROM zones WHERE rating >= $min_rating ORDER BY name;" >> "$output_file"
            ;;

        "csv")
            # Create CSV format
            echo "name,rating,verified" > "$output_file"
            eval "$db_cmd" <<< "SELECT name || ',' || rating || ',' || verified FROM zones WHERE rating >= $min_rating ORDER BY name;" >> "$output_file"
            ;;

        *)
            log_error "Unsupported format: $format"
            exit 1
            ;;
    esac

    local count=$(eval "$db_cmd" <<< "SELECT COUNT(*) FROM zones WHERE rating >= $min_rating;")
    log_success "Exported $count zone ratings to: $output_file"
}

create_backup() {
    local backup_file="$BACKUP_DIR/zone_ratings_backup_${DATE}.${OUTPUT_FORMAT}"

    export_zone_ratings "$backup_file" "$OUTPUT_FORMAT" "$MIN_RATING"

    # Create metadata file
    local metadata_file="${backup_file}.meta"
    cat << EOF > "$metadata_file"
{
    "backup_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "source": "$([ "$USE_DOCKER" == "true" ] && echo "docker:$CONTAINER_NAME" || echo "local:$DB_PATH")",
    "format": "$OUTPUT_FORMAT",
    "min_rating": $MIN_RATING,
    "total_zones": $(eval "$(get_db_connection)" <<< "SELECT COUNT(*) FROM zones;"),
    "rated_zones": $(eval "$(get_db_connection)" <<< "SELECT COUNT(*) FROM zones WHERE rating >= $MIN_RATING;")
}
EOF

    log_success "Backup completed: $backup_file"
    log_info "Metadata saved: $metadata_file"
    echo "$backup_file"
}

apply_zone_ratings() {
    local backup_file="$1"

    if [[ ! -f "$backup_file" ]]; then
        log_error "Backup file not found: $backup_file"
        exit 1
    fi

    log_info "Applying zone ratings from: $backup_file"

    local db_cmd=$(get_db_connection)
    local format="${backup_file##*.}"

    case $format in
        "json")
            # Parse JSON and apply ratings
            if command -v jq &> /dev/null; then
                local temp_sql="/tmp/zone_ratings_apply_$$.sql"
                jq -r '.[] | "UPDATE zones SET rating = \(.rating), verified = \(.verified) WHERE name = '\''\(.name)'\''; "' "$backup_file" > "$temp_sql"

                log_info "Applying $(wc -l < "$temp_sql") zone rating updates..."
                eval "$db_cmd" < "$temp_sql"
                rm -f "$temp_sql"
            else
                log_error "jq is required to apply JSON backup files"
                exit 1
            fi
            ;;

        "sql")
            # Apply SQL directly
            log_info "Applying SQL zone rating updates..."
            eval "$db_cmd" < "$backup_file"
            ;;

        "csv")
            # Parse CSV and apply ratings
            local temp_sql="/tmp/zone_ratings_apply_$$.sql"
            tail -n +2 "$backup_file" | while IFS=',' read -r name rating verified; do
                echo "UPDATE zones SET rating = $rating, verified = $verified WHERE name = '$name';"
            done > "$temp_sql"

            log_info "Applying $(wc -l < "$temp_sql") zone rating updates..."
            eval "$db_cmd" < "$temp_sql"
            rm -f "$temp_sql"
            ;;

        *)
            log_error "Unsupported backup file format: $format"
            exit 1
            ;;
    esac

    log_success "Zone ratings applied successfully"
}

create_auto_migration() {
    local backup_file=$(create_backup)
    local migration_script="$MIGRATIONS_DIR/auto_migrate_zone_ratings_${DATE}.sh"

    log_info "Creating auto-migration script..."

    cat << 'EOF' > "$migration_script"
#!/bin/bash

# Auto-generated Zone Ratings Migration Script
# This script can be run during Docker rebuilds to reapply zone ratings

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKUP_FILE="$(dirname "$SCRIPT_DIR")/backups/zone_ratings/zone_ratings_backup_${DATE}.${OUTPUT_FORMAT}"
EOF

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
EOF

    chmod +x "$migration_script"

    log_success "Auto-migration script created: $migration_script"
    log_info "To use this script during Docker rebuilds, add it to your Docker startup process"
    log_info "Example: Add 'RUN $migration_script' to your Dockerfile or call it from your entrypoint script"

    echo "$migration_script"
}

list_backups() {
    log_info "Available zone rating backups:"
    echo ""

    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_warning "Backup directory does not exist: $BACKUP_DIR"
        return 1
    fi

    local backup_files=($(find "$BACKUP_DIR" -name "zone_ratings_backup_*" -type f | grep -E "\.(json|sql|csv)$" | sort))

    if [[ ${#backup_files[@]} -eq 0 ]]; then
        log_warning "No backup files found in $BACKUP_DIR"
        return 1
    fi

    for i in "${!backup_files[@]}"; do
        local file="${backup_files[i]}"
        local size=$(du -h "$file" | cut -f1)
        local date=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M:%S" "$file" 2>/dev/null || stat -c "%y" "$file" 2>/dev/null || echo "unknown")
        local meta_file="${file}.meta"
        local zones_count="unknown"

        if [[ -f "$meta_file" ]]; then
            if command -v jq >/dev/null 2>&1; then
                zones_count=$(jq -r '.rated_zones // "unknown"' "$meta_file" 2>/dev/null || echo "unknown")
            fi
        fi

        printf "%2d. %-40s %8s  %s  (%s zones)\n" $((i+1)) "$(basename "$file")" "$size" "$date" "$zones_count"
    done

    echo ""
    log_info "Total: ${#backup_files[@]} backup files"
}

show_status() {
    log_info "Zone Ratings Status"
    echo "==================="
    echo ""

    local db_cmd=$(get_db_connection)

    # Basic statistics
    local total_zones=$(eval "$db_cmd" <<< "SELECT COUNT(*) FROM zones;")
    local rated_zones=$(eval "$db_cmd" <<< "SELECT COUNT(*) FROM zones WHERE rating > 0;")
    local verified_zones=$(eval "$db_cmd" <<< "SELECT COUNT(*) FROM zones WHERE verified = 1;")
    local avg_rating=$(eval "$db_cmd" <<< "SELECT ROUND(AVG(rating), 2) FROM zones WHERE rating > 0;")

    echo "Total zones: $total_zones"
    echo "Zones with ratings: $rated_zones"
    echo "Verified zones: $verified_zones"
    echo "Average rating: $avg_rating"
    echo ""

    # Rating distribution
    echo "Rating Distribution:"
    eval "$db_cmd" <<< "SELECT 'Rating ' || rating || ': ' || COUNT(*) || ' zones' FROM zones WHERE rating > 0 GROUP BY rating ORDER BY rating DESC;"
    echo ""

    # Recent changes (if we have timestamp info)
    echo "Recent Activity:"
    eval "$db_cmd" <<< "SELECT 'Recent zones: ' || COUNT(*) FROM zones WHERE created_at > datetime('now', '-7 days');"
}

# Default values
USE_DOCKER="false"
FORCE="false"
OUTPUT_FORMAT="json"
MIN_RATING=1
BACKUP_FILE=""
COMMAND="backup"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            print_usage
            exit 0
            ;;
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
        -o|--output)
            BACKUP_DIR="$2"
            shift 2
            ;;
        -f|--file)
            BACKUP_FILE="$2"
            shift 2
            ;;
        --force)
            FORCE="true"
            shift
            ;;
        --format)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        --min-rating)
            MIN_RATING="$2"
            shift 2
            ;;
        backup|migrate|auto-migrate|list-backups|status)
            COMMAND="$1"
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Main execution
main() {
    log_info "Zone Ratings Backup/Migration Utility"
    log_info "Command: $COMMAND"
    log_info "Timestamp: $(date)"

    # Check dependencies
    check_dependencies

    case $COMMAND in
        "backup")
            create_directories
            create_backup
            ;;
        "migrate")
            if [[ -z "$BACKUP_FILE" ]]; then
                log_error "Migration requires --file option"
                exit 1
            fi
            if [[ "$FORCE" != "true" ]]; then
                read -p "Apply zone ratings from $BACKUP_FILE? (yes/no): " confirmation
                case $confirmation in
                    yes|YES|y|Y) ;;
                    *) log_info "Migration cancelled"; exit 0 ;;
                esac
            fi
            apply_zone_ratings "$BACKUP_FILE"
            ;;
        "auto-migrate")
            create_directories
            create_auto_migration
            ;;
        "list-backups")
            list_backups
            ;;
        "status")
            show_status
            ;;
        *)
            log_error "Unknown command: $COMMAND"
            print_usage
            exit 1
            ;;
    esac

    log_success "Operation completed successfully!"
}

# Run main function
main "$@"
