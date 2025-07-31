#!/bin/bash

# Enhanced Docker Rebuild Script with Zone Ratings Backup/Restore
# This script handles the complete rebuild process while preserving zone ratings

set -e  # Exit on any error

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BACKUP_DIR="$PROJECT_ROOT/backups/zone_ratings"
MIGRATIONS_DIR="$PROJECT_ROOT/migrations/zone_ratings"
CONTAINER_NAME="eq_rng-app-1"
DATE=$(date +"%Y%m%d_%H%M%S")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[REBUILD]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[REBUILD]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[REBUILD]${NC} $1"
}

log_error() {
    echo -e "${RED}[REBUILD]${NC} $1"
}

log_step() {
    echo -e "${CYAN}[STEP]${NC} $1"
    echo "=============================================="
}

print_usage() {
    echo "Enhanced Docker Rebuild Script with Zone Ratings Management"
    echo "=========================================================="
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help              Show this help message"
    echo "  --no-backup             Skip zone ratings backup before rebuild"
    echo "  --no-restore            Skip zone ratings restoration after rebuild"
    echo "  --no-cache              Build Docker image without cache"
    echo "  --skip-git              Skip git pull"
    echo "  --keep-container        Don't remove the old container"
    echo "  --backup-only           Only create zone ratings backup, don't rebuild"
    echo "  --restore-only FILE     Only restore zone ratings from specified backup"
    echo "  --dry-run               Show what would be done without executing"
    echo "  -f, --force             Force operations without confirmation"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Full rebuild with zone ratings preservation"
    echo "  $0 --no-cache                        # Rebuild without Docker cache"
    echo "  $0 --backup-only                     # Only backup current zone ratings"
    echo "  $0 --restore-only backup_file.json   # Only restore from specific backup"
    echo "  $0 --dry-run                         # Preview what will be done"
}

check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check if we're in the right directory
    if [[ ! -f "$PROJECT_ROOT/docker-compose.yml" ]]; then
        log_error "docker-compose.yml not found. Please run this script from the project root or utils directory."
        exit 1
    fi

    # Check required tools
    local missing_tools=()

    if ! command -v docker &> /dev/null; then
        missing_tools+=("docker")
    fi

    if ! command -v docker-compose &> /dev/null; then
        missing_tools+=("docker-compose")
    fi

    if ! command -v git &> /dev/null && [[ "$SKIP_GIT" != "true" ]]; then
        missing_tools+=("git")
    fi

    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        exit 1
    fi

    # Check if zone ratings backup script exists
    if [[ ! -f "$SCRIPT_DIR/backup_zone_ratings.sh" ]]; then
        log_warning "Zone ratings backup script not found. Zone ratings preservation will be limited."
        BACKUP_SCRIPT_AVAILABLE=false
    else
        BACKUP_SCRIPT_AVAILABLE=true
    fi

    log_success "Prerequisites check completed"
}

create_directories() {
    log_info "Creating necessary directories..."

    mkdir -p "$BACKUP_DIR"
    mkdir -p "$MIGRATIONS_DIR"

    log_success "Directories created"
}

backup_zone_ratings() {
    if [[ "$NO_BACKUP" == "true" ]]; then
        log_info "Skipping zone ratings backup (--no-backup specified)"
        return 0
    fi

    log_step "BACKING UP ZONE RATINGS"

    # Check if container is running
    if ! docker ps --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        log_warning "Container not running, cannot backup current zone ratings"
        return 0
    fi

    # Use the comprehensive backup script if available
    if [[ "$BACKUP_SCRIPT_AVAILABLE" == "true" ]]; then
        log_info "Using comprehensive zone ratings backup script..."

        if [[ "$DRY_RUN" == "true" ]]; then
            log_info "[DRY RUN] Would run: $SCRIPT_DIR/backup_zone_ratings.sh backup --docker --format json"
        else
            BACKUP_FILE=$("$SCRIPT_DIR/backup_zone_ratings.sh" backup --docker --format json)
            if [[ $? -eq 0 && -n "$BACKUP_FILE" ]]; then
                log_success "Zone ratings backed up to: $BACKUP_FILE"
                echo "$BACKUP_FILE" > "$BACKUP_DIR/.last_backup"
            else
                log_error "Zone ratings backup failed"
                return 1
            fi
        fi
    else
        # Fallback simple backup
        log_info "Using simple zone ratings backup..."

        local backup_file="$BACKUP_DIR/zone_ratings_rebuild_${DATE}.sql"

        if [[ "$DRY_RUN" == "true" ]]; then
            log_info "[DRY RUN] Would create backup: $backup_file"
        else
            docker exec "$CONTAINER_NAME" sqlite3 /opt/eq_rng/data/zones.db \
                "SELECT 'UPDATE zones SET rating = ' || rating || ', verified = ' || verified || ' WHERE name = ''' || name || ''';' FROM zones WHERE rating > 0 ORDER BY name;" \
                > "$backup_file" 2>/dev/null || true

            if [[ -f "$backup_file" && -s "$backup_file" ]]; then
                log_success "Zone ratings backed up to: $backup_file"
                echo "$backup_file" > "$BACKUP_DIR/.last_backup"
            else
                log_warning "No zone ratings found to backup"
                rm -f "$backup_file" 2>/dev/null || true
            fi
        fi
    fi
}

git_pull() {
    if [[ "$SKIP_GIT" == "true" ]]; then
        log_info "Skipping git pull (--skip-git specified)"
        return 0
    fi

    log_step "UPDATING SOURCE CODE"

    cd "$PROJECT_ROOT"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would run: git pull"
    else
        log_info "Pulling latest changes from git..."
        git pull
        log_success "Source code updated"
    fi
}

stop_containers() {
    log_step "STOPPING CONTAINERS"

    cd "$PROJECT_ROOT"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would run: docker-compose down"
    else
        log_info "Stopping Docker containers..."
        docker-compose down
        log_success "Containers stopped"
    fi
}

build_image() {
    log_step "BUILDING DOCKER IMAGE"

    cd "$PROJECT_ROOT"

    local build_args="docker-compose build"

    if [[ "$NO_CACHE" == "true" ]]; then
        build_args="$build_args --no-cache"
    fi

    build_args="$build_args app"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would run: $build_args"
    else
        log_info "Building Docker image..."
        log_info "Build command: $build_args"

        export DOCKER_BUILDKIT=1
        eval "$build_args"

        log_success "Docker image built successfully"
    fi
}

start_containers() {
    log_step "STARTING CONTAINERS"

    cd "$PROJECT_ROOT"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would run: docker-compose up -d"
    else
        log_info "Starting Docker containers..."
        docker-compose up -d

        # Wait a moment for container to start
        sleep 5

        log_success "Containers started"
    fi
}

wait_for_application() {
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would wait for application to be ready"
        return 0
    fi

    log_info "Waiting for application to be ready..."

    local timeout=60
    local counter=0

    while [[ $counter -lt $timeout ]]; do
        if docker ps --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
            # Check if the application is responding
            if docker exec "$CONTAINER_NAME" sqlite3 /opt/eq_rng/data/zones.db "SELECT COUNT(*) FROM zones;" >/dev/null 2>&1; then
                log_success "Application is ready"
                return 0
            fi
        fi

        counter=$((counter + 1))
        sleep 1

        if [[ $((counter % 10)) -eq 0 ]]; then
            log_info "Still waiting for application... ($counter/$timeout seconds)"
        fi
    done

    log_warning "Application may not be fully ready after $timeout seconds"
    return 1
}

restore_zone_ratings() {
    if [[ "$NO_RESTORE" == "true" ]]; then
        log_info "Skipping zone ratings restoration (--no-restore specified)"
        return 0
    fi

    log_step "RESTORING ZONE RATINGS"

    # Find the backup file to restore
    local backup_file=""

    if [[ -n "$RESTORE_FILE" ]]; then
        backup_file="$RESTORE_FILE"
    elif [[ -f "$BACKUP_DIR/.last_backup" ]]; then
        backup_file=$(cat "$BACKUP_DIR/.last_backup")
    else
        # Find the most recent backup
        backup_file=$(find "$BACKUP_DIR" -name "zone_ratings_*" -type f | sort | tail -1)
    fi

    if [[ -z "$backup_file" || ! -f "$backup_file" ]]; then
        log_warning "No zone ratings backup found to restore"
        return 0
    fi

    log_info "Restoring zone ratings from: $(basename "$backup_file")"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would restore zone ratings from: $backup_file"
        return 0
    fi

    # Wait for application to be ready
    wait_for_application

    # Use the comprehensive restore script if available
    if [[ "$BACKUP_SCRIPT_AVAILABLE" == "true" ]]; then
        log_info "Using comprehensive zone ratings restore..."
        "$SCRIPT_DIR/backup_zone_ratings.sh" migrate --docker --file "$backup_file" --force
    else
        # Fallback simple restore
        log_info "Using simple zone ratings restore..."

        if [[ "${backup_file##*.}" == "sql" ]]; then
            docker exec "$CONTAINER_NAME" sqlite3 /opt/eq_rng/data/zones.db < "$backup_file"
        else
            log_error "Unsupported backup file format for simple restore: ${backup_file##*.}"
            return 1
        fi
    fi

    log_success "Zone ratings restoration completed"
}

show_status() {
    log_step "DEPLOYMENT STATUS"

    cd "$PROJECT_ROOT"

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would show container status and logs"
        return 0
    fi

    log_info "Container status:"
    docker ps | grep -E "(CONTAINER|$CONTAINER_NAME)" || true

    echo ""
    log_info "Recent application logs:"
    docker-compose logs --tail=20 app || true

    echo ""
    if docker ps --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        log_info "Database status:"
        local zone_count=$(docker exec "$CONTAINER_NAME" sqlite3 /opt/eq_rng/data/zones.db "SELECT COUNT(*) FROM zones;" 2>/dev/null || echo "Error")
        local rated_zones=$(docker exec "$CONTAINER_NAME" sqlite3 /opt/eq_rng/data/zones.db "SELECT COUNT(*) FROM zones WHERE rating > 0;" 2>/dev/null || echo "Error")

        echo "Total zones: $zone_count"
        echo "Zones with ratings: $rated_zones"
    else
        log_warning "Container not running, cannot check database status"
    fi
}

cleanup_old_backups() {
    log_info "Cleaning up old backups..."

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would clean up old backups (keeping last 10)"
        return 0
    fi

    # Keep only the last 10 backups
    find "$BACKUP_DIR" -name "zone_ratings_*" -type f | sort | head -n -10 | xargs rm -f 2>/dev/null || true

    log_success "Old backups cleaned up"
}

# Default values
NO_BACKUP=false
NO_RESTORE=false
NO_CACHE=false
SKIP_GIT=false
KEEP_CONTAINER=false
BACKUP_ONLY=false
RESTORE_ONLY=false
DRY_RUN=false
FORCE=false
RESTORE_FILE=""
BACKUP_SCRIPT_AVAILABLE=true

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            print_usage
            exit 0
            ;;
        --no-backup)
            NO_BACKUP=true
            shift
            ;;
        --no-restore)
            NO_RESTORE=true
            shift
            ;;
        --no-cache)
            NO_CACHE=true
            shift
            ;;
        --skip-git)
            SKIP_GIT=true
            shift
            ;;
        --keep-container)
            KEEP_CONTAINER=true
            shift
            ;;
        --backup-only)
            BACKUP_ONLY=true
            shift
            ;;
        --restore-only)
            RESTORE_ONLY=true
            RESTORE_FILE="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -f|--force)
            FORCE=true
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
    echo "=============================================="
    echo "Enhanced Docker Rebuild with Zone Ratings"
    echo "=============================================="
    echo "Started at: $(date)"
    echo ""

    if [[ "$DRY_RUN" == "true" ]]; then
        log_warning "DRY RUN MODE - No changes will be made"
        echo ""
    fi

    # Check prerequisites
    check_prerequisites

    # Create directories
    create_directories

    # Handle special modes
    if [[ "$BACKUP_ONLY" == "true" ]]; then
        backup_zone_ratings
        cleanup_old_backups
        log_success "Backup completed successfully!"
        exit 0
    fi

    if [[ "$RESTORE_ONLY" == "true" ]]; then
        if [[ -z "$RESTORE_FILE" ]]; then
            log_error "Restore file must be specified with --restore-only"
            exit 1
        fi
        restore_zone_ratings
        log_success "Restore completed successfully!"
        exit 0
    fi

    # Confirmation for full rebuild
    if [[ "$FORCE" != "true" && "$DRY_RUN" != "true" ]]; then
        echo "This will rebuild the Docker container and may cause temporary downtime."
        echo "Zone ratings will be preserved automatically."
        echo ""
        read -p "Continue with rebuild? (yes/no): " confirmation
        case $confirmation in
            yes|YES|y|Y)
                log_info "Proceeding with rebuild..."
                ;;
            *)
                log_info "Rebuild cancelled by user"
                exit 0
                ;;
        esac
        echo ""
    fi

    # Full rebuild sequence
    log_info "Starting full rebuild sequence..."
    echo ""

    # 1. Backup current zone ratings
    backup_zone_ratings

    # 2. Update source code
    git_pull

    # 3. Stop containers
    stop_containers

    # 4. Build new image
    build_image

    # 5. Start containers
    start_containers

    # 6. Restore zone ratings
    restore_zone_ratings

    # 7. Show status
    show_status

    # 8. Cleanup
    cleanup_old_backups

    echo ""
    log_success "=============================================="
    log_success "Rebuild completed successfully!"
    log_success "=============================================="
    echo "Completed at: $(date)"

    if [[ "$DRY_RUN" != "true" ]]; then
        echo ""
        echo "Your application should now be running with preserved zone ratings."
        echo "Check the status above to verify everything is working correctly."
    fi
}

# Run main function
main "$@"
