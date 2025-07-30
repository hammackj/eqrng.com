#!/bin/bash

# Database Backup Utility for eq_rng.com
# Supports both local and Docker containerized environments

set -e  # Exit on any error

# Configuration
DB_PATH="data/zones.db"
BACKUP_DIR="backups/database"
DATE=$(date +"%Y%m%d_%H%M%S")
BACKUP_NAME="zones_backup_${DATE}.db"
CONTAINER_NAME="eq_rng-app-1"  # Default Docker Compose container name

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
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help          Show this help message"
    echo "  -d, --docker        Backup from Docker container"
    echo "  -l, --local         Backup from local filesystem (default)"
    echo "  -o, --output DIR    Specify backup directory (default: backups/database)"
    echo "  -n, --name NAME     Specify backup filename (default: zones_backup_YYYYMMDD_HHMMSS.db)"
    echo "  -c, --container NAME Docker container name (default: eq_rng-app-1)"
    echo "  -k, --keep NUM      Keep only the last NUM backups (cleanup old ones)"
    echo "  --compress          Compress the backup with gzip"
    echo ""
    echo "Examples:"
    echo "  $0                           # Create local backup"
    echo "  $0 --docker                  # Create backup from Docker container"
    echo "  $0 --keep 10                 # Create backup and keep only last 10"
    echo "  $0 --compress                # Create compressed backup"
    echo "  $0 -d -k 5 --compress       # Docker backup, keep 5, compress"
}

check_dependencies() {
    if [[ "$USE_DOCKER" == "true" ]]; then
        if ! command -v docker &> /dev/null; then
            log_error "Docker is not installed or not in PATH"
            exit 1
        fi
    fi

    if [[ "$COMPRESS" == "true" ]]; then
        if ! command -v gzip &> /dev/null; then
            log_error "gzip is not installed or not in PATH"
            exit 1
        fi
    fi
}

create_backup_dir() {
    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_info "Creating backup directory: $BACKUP_DIR"
        mkdir -p "$BACKUP_DIR"
    fi
}

backup_local() {
    local source_path="$1"
    local backup_path="$2"

    if [[ ! -f "$source_path" ]]; then
        log_error "Database file not found: $source_path"
        exit 1
    fi

    log_info "Creating backup from local filesystem..."
    log_info "Source: $source_path"
    log_info "Destination: $backup_path"

    # Use sqlite3 to create a consistent backup
    if command -v sqlite3 &> /dev/null; then
        sqlite3 "$source_path" ".backup '$backup_path'"
        log_success "SQLite backup created successfully"
    else
        # Fallback to simple copy
        log_warning "sqlite3 not found, using file copy (may be inconsistent if DB is in use)"
        cp "$source_path" "$backup_path"
        log_success "File copy backup created"
    fi
}

backup_docker() {
    local container_path="/opt/eq_rng/$DB_PATH"
    local backup_path="$1"

    # Check if container is running
    if ! docker ps --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        log_error "Docker container '$CONTAINER_NAME' is not running"
        exit 1
    fi

    log_info "Creating backup from Docker container..."
    log_info "Container: $CONTAINER_NAME"
    log_info "Source: $container_path"
    log_info "Destination: $backup_path"

    # Try to use sqlite3 from within the container first
    if docker exec "$CONTAINER_NAME" which sqlite3 &> /dev/null; then
        docker exec "$CONTAINER_NAME" sqlite3 "$container_path" ".backup /tmp/backup.db"
        docker cp "${CONTAINER_NAME}:/tmp/backup.db" "$backup_path"
        docker exec "$CONTAINER_NAME" rm "/tmp/backup.db"
        log_success "SQLite backup from container created successfully"
    else
        # Fallback to docker cp
        log_warning "sqlite3 not found in container, using docker cp (may be inconsistent if DB is in use)"
        docker cp "${CONTAINER_NAME}:${container_path}" "$backup_path"
        log_success "File copy backup from container created"
    fi
}

compress_backup() {
    local backup_path="$1"
    local compressed_path="${backup_path}.gz"

    log_info "Compressing backup..." >&2
    gzip "$backup_path"
    log_success "Backup compressed: $compressed_path" >&2
    echo "$compressed_path"
}

cleanup_old_backups() {
    local keep_count="$1"

    log_info "Cleaning up old backups (keeping last $keep_count)..."

    # Find all backup files and sort by modification time
    local backup_files=($(find "$BACKUP_DIR" \( -name "zones_backup_*.db" -o -name "zones_backup_*.db.gz" \) -type f -exec stat -f "%m %N" {} \; | sort -n | cut -d' ' -f2-))
    local total_files=${#backup_files[@]}

    if [[ $total_files -gt $keep_count ]]; then
        local files_to_delete=$((total_files - keep_count))
        log_info "Found $total_files backups, removing oldest $files_to_delete"

        for ((i=0; i<files_to_delete; i++)); do
            log_info "Removing: ${backup_files[i]}"
            rm -f "${backup_files[i]}"
        done

        log_success "Cleanup completed"
    else
        log_info "No cleanup needed (found $total_files backups, keeping $keep_count)"
    fi
}

verify_backup() {
    local backup_path="$1"

    # If it's compressed, we can't easily verify the SQLite integrity
    if [[ "$backup_path" == *.gz ]]; then
        if [[ -f "$backup_path" && -s "$backup_path" ]]; then
            log_success "Compressed backup file exists and is not empty"
            return 0
        else
            log_error "Compressed backup file is missing or empty"
            return 1
        fi
    fi

    # Verify SQLite integrity
    if command -v sqlite3 &> /dev/null; then
        if sqlite3 "$backup_path" "PRAGMA integrity_check;" | grep -q "ok"; then
            log_success "Backup integrity verified"
            return 0
        else
            log_error "Backup integrity check failed"
            return 1
        fi
    else
        # Basic file check
        if [[ -f "$backup_path" && -s "$backup_path" ]]; then
            log_success "Backup file exists and is not empty"
            return 0
        else
            log_error "Backup file is missing or empty"
            return 1
        fi
    fi
}

# Default values
USE_DOCKER="false"
COMPRESS="false"
KEEP_BACKUPS=""

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
        -o|--output)
            BACKUP_DIR="$2"
            shift 2
            ;;
        -n|--name)
            BACKUP_NAME="$2"
            shift 2
            ;;
        -c|--container)
            CONTAINER_NAME="$2"
            shift 2
            ;;
        -k|--keep)
            KEEP_BACKUPS="$2"
            shift 2
            ;;
        --compress)
            COMPRESS="true"
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
    log_info "Starting database backup process..."
    log_info "Timestamp: $(date)"

    # Check dependencies
    check_dependencies

    # Create backup directory
    create_backup_dir

    # Determine backup path
    local backup_path="$BACKUP_DIR/$BACKUP_NAME"

    # Perform backup
    if [[ "$USE_DOCKER" == "true" ]]; then
        backup_docker "$backup_path"
    else
        backup_local "$DB_PATH" "$backup_path"
    fi

    # Compress if requested
    if [[ "$COMPRESS" == "true" ]]; then
        backup_path=$(compress_backup "$backup_path")
    fi

    # Verify backup
    if verify_backup "$backup_path"; then
        local backup_size=$(du -h "$backup_path" | cut -f1)
        log_success "Backup completed successfully!"
        log_info "Backup location: $backup_path"
        log_info "Backup size: $backup_size"
    else
        log_error "Backup verification failed!"
        exit 1
    fi

    # Cleanup old backups if requested
    if [[ -n "$KEEP_BACKUPS" ]]; then
        cleanup_old_backups "$KEEP_BACKUPS"
    fi

    log_success "Database backup process completed!"
}

# Run main function
main "$@"
