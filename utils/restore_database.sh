#!/bin/bash

# Database Restore Utility for eq_rng.com
# Supports both local and Docker containerized environments

set -e  # Exit on any error

# Configuration
DB_PATH="data/zones.db"
BACKUP_DIR="backups/database"
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
    echo "Usage: $0 [OPTIONS] BACKUP_FILE"
    echo ""
    echo "Options:"
    echo "  -h, --help          Show this help message"
    echo "  -d, --docker        Restore to Docker container"
    echo "  -l, --local         Restore to local filesystem (default)"
    echo "  -c, --container NAME Docker container name (default: eq_rng-app-1)"
    echo "  -f, --force         Force restore without confirmation"
    echo "  --list              List available backup files"
    echo "  --latest            Restore from the most recent backup"
    echo ""
    echo "Arguments:"
    echo "  BACKUP_FILE         Path to backup file to restore from"
    echo ""
    echo "Examples:"
    echo "  $0 backups/database/zones_backup_20231215_143022.db"
    echo "  $0 --latest                                          # Restore latest backup"
    echo "  $0 --docker --latest                                 # Restore latest to Docker"
    echo "  $0 --list                                            # List available backups"
}

check_dependencies() {
    if [[ "$USE_DOCKER" == "true" ]]; then
        if ! command -v docker &> /dev/null; then
            log_error "Docker is not installed or not in PATH"
            exit 1
        fi
    fi
}

list_backups() {
    log_info "Available backup files:"
    echo ""

    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_warning "Backup directory does not exist: $BACKUP_DIR"
        return 1
    fi

    local backup_files=($(find "$BACKUP_DIR" \( -name "zones_backup_*.db" -o -name "zones_backup_*.db.gz" \) -type f -exec stat -f "%m %N" {} \; | sort -n | cut -d' ' -f2-))

    if [[ ${#backup_files[@]} -eq 0 ]]; then
        log_warning "No backup files found in $BACKUP_DIR"
        return 1
    fi

    for i in "${!backup_files[@]}"; do
        local file="${backup_files[i]}"
        local size=$(du -h "$file" | cut -f1)
        local date=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M:%S" "$file")
        printf "%2d. %-50s %8s  %s\n" $((i+1)) "$(basename "$file")" "$size" "$date"
    done

    echo ""
    log_info "Total: ${#backup_files[@]} backup files"
}

get_latest_backup() {
    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_error "Backup directory does not exist: $BACKUP_DIR"
        exit 1
    fi

    local latest_backup=$(find "$BACKUP_DIR" \( -name "zones_backup_*.db" -o -name "zones_backup_*.db.gz" \) -type f -exec stat -f "%m %N" {} \; | sort -rn | head -1 | cut -d' ' -f2-)

    if [[ -z "$latest_backup" ]]; then
        log_error "No backup files found in $BACKUP_DIR"
        exit 1
    fi

    echo "$latest_backup"
}

verify_backup_file() {
    local backup_file="$1"

    if [[ ! -f "$backup_file" ]]; then
        log_error "Backup file does not exist: $backup_file"
        exit 1
    fi

    # If it's compressed, decompress for verification
    local temp_file=""
    local file_to_check="$backup_file"

    if [[ "$backup_file" == *.gz ]]; then
        log_info "Decompressing backup for verification..."
        temp_file="/tmp/restore_temp_$(date +%s).db"
        gunzip -c "$backup_file" > "$temp_file"
        file_to_check="$temp_file"
    fi

    # Verify SQLite integrity
    if command -v sqlite3 &> /dev/null; then
        if sqlite3 "$file_to_check" "PRAGMA integrity_check;" | grep -q "ok"; then
            log_success "Backup file integrity verified"
        else
            log_error "Backup file integrity check failed"
            [[ -n "$temp_file" ]] && rm -f "$temp_file"
            exit 1
        fi
    else
        log_warning "sqlite3 not available, skipping integrity check"
    fi

    # Clean up temp file if created
    [[ -n "$temp_file" ]] && rm -f "$temp_file"
}

create_backup_before_restore() {
    local backup_name="zones_backup_before_restore_$(date +%Y%m%d_%H%M%S).db"
    local safety_backup_path="$BACKUP_DIR/$backup_name"

    log_info "Creating safety backup before restore..."

    if [[ "$USE_DOCKER" == "true" ]]; then
        # Check if container is running
        if ! docker ps --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
            log_error "Docker container '$CONTAINER_NAME' is not running"
            exit 1
        fi

        local container_path="/opt/eq_rng/$DB_PATH"
        docker cp "${CONTAINER_NAME}:${container_path}" "$safety_backup_path"
    else
        if [[ ! -f "$DB_PATH" ]]; then
            log_warning "Current database file does not exist: $DB_PATH"
            return 0
        fi
        cp "$DB_PATH" "$safety_backup_path"
    fi

    log_success "Safety backup created: $safety_backup_path"
}

restore_local() {
    local backup_file="$1"
    local target_path="$DB_PATH"

    # Handle compressed files
    if [[ "$backup_file" == *.gz ]]; then
        log_info "Decompressing and restoring to local filesystem..."
        gunzip -c "$backup_file" > "$target_path"
    else
        log_info "Restoring to local filesystem..."
        cp "$backup_file" "$target_path"
    fi

    log_success "Database restored to: $target_path"
}

restore_docker() {
    local backup_file="$1"
    local container_path="/opt/eq_rng/$DB_PATH"

    # Check if container is running
    if ! docker ps --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        log_error "Docker container '$CONTAINER_NAME' is not running"
        exit 1
    fi

    local temp_file="/tmp/restore_$(date +%s).db"

    # Handle compressed files
    if [[ "$backup_file" == *.gz ]]; then
        log_info "Decompressing backup..."
        gunzip -c "$backup_file" > "$temp_file"
    else
        cp "$backup_file" "$temp_file"
    fi

    log_info "Restoring to Docker container..."
    docker cp "$temp_file" "${CONTAINER_NAME}:${container_path}"

    # Clean up temp file
    rm -f "$temp_file"

    log_success "Database restored to container: $container_path"
}

confirm_restore() {
    local backup_file="$1"

    echo ""
    log_warning "RESTORE CONFIRMATION"
    echo "=================="
    echo "This will REPLACE the current database with the backup."
    echo "Current database: $DB_PATH"
    echo "Backup file: $backup_file"
    echo "Target: $([ "$USE_DOCKER" == "true" ] && echo "Docker container ($CONTAINER_NAME)" || echo "Local filesystem")"
    echo ""

    if [[ "$FORCE" != "true" ]]; then
        read -p "Are you sure you want to proceed? (yes/no): " confirmation
        case $confirmation in
            yes|YES|y|Y)
                log_info "Proceeding with restore..."
                ;;
            *)
                log_info "Restore cancelled by user"
                exit 0
                ;;
        esac
    else
        log_info "Force mode enabled, proceeding with restore..."
    fi
}

# Default values
USE_DOCKER="false"
FORCE="false"
BACKUP_FILE=""
LIST_BACKUPS="false"
USE_LATEST="false"

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
        -f|--force)
            FORCE="true"
            shift
            ;;
        --list)
            LIST_BACKUPS="true"
            shift
            ;;
        --latest)
            USE_LATEST="true"
            shift
            ;;
        -*)
            log_error "Unknown option: $1"
            print_usage
            exit 1
            ;;
        *)
            if [[ -z "$BACKUP_FILE" ]]; then
                BACKUP_FILE="$1"
            else
                log_error "Multiple backup files specified"
                print_usage
                exit 1
            fi
            shift
            ;;
    esac
done

# Main execution
main() {
    log_info "Starting database restore process..."
    log_info "Timestamp: $(date)"

    # Handle list command
    if [[ "$LIST_BACKUPS" == "true" ]]; then
        list_backups
        exit 0
    fi

    # Handle latest backup
    if [[ "$USE_LATEST" == "true" ]]; then
        BACKUP_FILE=$(get_latest_backup)
        log_info "Using latest backup: $BACKUP_FILE"
    fi

    # Check if backup file is specified
    if [[ -z "$BACKUP_FILE" ]]; then
        log_error "No backup file specified"
        echo ""
        print_usage
        exit 1
    fi

    # Check dependencies
    check_dependencies

    # Verify backup file
    verify_backup_file "$BACKUP_FILE"

    # Confirm restore
    confirm_restore "$BACKUP_FILE"

    # Create safety backup
    create_backup_before_restore

    # Perform restore
    if [[ "$USE_DOCKER" == "true" ]]; then
        restore_docker "$BACKUP_FILE"
    else
        restore_local "$BACKUP_FILE"
    fi

    log_success "Database restore completed successfully!"
    log_info "Please restart your application to ensure it picks up the restored database"
}

# Run main function
main "$@"
