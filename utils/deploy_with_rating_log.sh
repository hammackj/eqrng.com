#!/bin/bash

# Simple Deployment Script with Rating Transaction Log for eq_rng.com
# This script extracts rating transaction logs before deployment and applies them after

set -e  # Exit on any error

# Configuration
OLD_CONTAINER="${OLD_CONTAINER:-eq_rng-app-1}"
BACKUP_DIR="${BACKUP_DIR:-backups/rating_transactions}"
DOCKER_COMPOSE_FILE="${DOCKER_COMPOSE_FILE:-docker-compose.yml}"
LOG_FILE="data/rating_transaction.log"
DATE=$(date +"%Y%m%d_%H%M%S")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[DEPLOY]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[DEPLOY]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[DEPLOY]${NC} $1"
}

log_error() {
    echo -e "${RED}[DEPLOY]${NC} $1"
}

print_usage() {
    echo "Simple Deployment with Rating Transaction Log"
    echo "============================================"
    echo ""
    echo "This script performs deployment while preserving rating data:"
    echo "1. Extract rating transaction log from old container"
    echo "2. Stop old container and deploy new one"
    echo "3. Apply rating transaction log to new container"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -c, --container NAME     Old container name (default: $OLD_CONTAINER)"
    echo "  -b, --backup-dir DIR     Backup directory (default: $BACKUP_DIR)"
    echo "  -f, --compose-file FILE  Docker compose file (default: $DOCKER_COMPOSE_FILE)"
    echo "  --skip-extract          Skip transaction log extraction"
    echo "  --skip-apply            Skip transaction log application"
    echo "  --dry-run               Show what would be done without executing"
    echo "  -h, --help              Show this help message"
}

# Parse command line arguments
SKIP_EXTRACT=false
SKIP_APPLY=false
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -c|--container)
            OLD_CONTAINER="$2"
            shift 2
            ;;
        -b|--backup-dir)
            BACKUP_DIR="$2"
            shift 2
            ;;
        -f|--compose-file)
            DOCKER_COMPOSE_FILE="$2"
            shift 2
            ;;
        --skip-extract)
            SKIP_EXTRACT=true
            shift
            ;;
        --skip-apply)
            SKIP_APPLY=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Ensure backup directory exists
mkdir -p "$BACKUP_DIR"

# Function to check if container is running
check_container() {
    local container_name="$1"
    if docker ps --format "table {{.Names}}" | grep -q "^${container_name}$"; then
        return 0
    else
        return 1
    fi
}

# Extract rating transaction log
extract_rating_log() {
    if [ "$SKIP_EXTRACT" = true ]; then
        log_warning "Skipping rating transaction log extraction"
        return 0
    fi

    log_info "Step 1: Extracting rating transaction log from container: $OLD_CONTAINER"

    if [ "$DRY_RUN" = true ]; then
        log_info "[DRY RUN] Would extract transaction log to: $BACKUP_DIR/rating_transactions_$DATE.sql"
        return 0
    fi

    if ! check_container "$OLD_CONTAINER"; then
        log_warning "Old container not running, skipping extraction"
        return 0
    fi

    local output_file="$BACKUP_DIR/rating_transactions_$DATE.sql"

    # Check if transaction log exists in container
    if ! docker exec "$OLD_CONTAINER" test -f "$LOG_FILE"; then
        log_warning "No transaction log file found in container - creating empty backup"
        echo "-- No rating transactions found (extracted on $DATE)" > "$output_file"
        log_info "Empty transaction log created: $output_file"
        return 0
    fi

    # Copy the file from container
    if docker cp "$OLD_CONTAINER:$LOG_FILE" "$output_file"; then
        local line_count=$(wc -l < "$output_file" 2>/dev/null || echo "0")
        local sql_count=$(grep -v '^--' "$output_file" 2>/dev/null | grep -v '^$' | wc -l 2>/dev/null || echo "0")

        log_success "Extracted transaction log: $output_file"
        log_info "Contains: $line_count lines, $sql_count SQL statements"

        # Add header to file
        local temp_file="$output_file.tmp"
        cat > "$temp_file" << EOF
-- Rating Transaction Log extracted on $DATE
-- Source container: $OLD_CONTAINER
-- Contains $sql_count SQL statements
--
$(cat "$output_file")
EOF
        mv "$temp_file" "$output_file"

        # Create metadata
        cat > "$output_file.meta" << EOF
{
    "extract_timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "source_container": "$OLD_CONTAINER",
    "deployment_date": "$DATE",
    "line_count": $line_count,
    "sql_statements": $sql_count,
    "file": "$(basename "$output_file")"
}
EOF

    else
        log_error "Failed to extract transaction log from container"
        return 1
    fi
}

# Deploy new container
deploy_new_container() {
    log_info "Step 2: Deploying new container"

    if [ "$DRY_RUN" = true ]; then
        log_info "[DRY RUN] Would stop old container and deploy new one"
        return 0
    fi

    # Stop old container
    if check_container "$OLD_CONTAINER"; then
        log_info "Stopping old container: $OLD_CONTAINER"
        docker stop "$OLD_CONTAINER" || true
        docker rm "$OLD_CONTAINER" || true
    fi

    # Build and start new container
    log_info "Building and starting new container..."
    docker-compose -f "$DOCKER_COMPOSE_FILE" up --build -d

    # Wait for container to be ready
    log_info "Waiting for new container to be ready..."
    sleep 10

    # Get new container name
    local new_container=$(docker-compose -f "$DOCKER_COMPOSE_FILE" ps -q | head -n1)
    if [ -n "$new_container" ]; then
        local container_name=$(docker inspect --format='{{.Name}}' "$new_container" | sed 's/^.//')
        log_success "New container started: $container_name"

        # Update the container name for apply step
        NEW_CONTAINER="$container_name"
    else
        log_error "Could not determine new container name"
        return 1
    fi
}

# Apply rating transaction log
apply_rating_log() {
    if [ "$SKIP_APPLY" = true ]; then
        log_warning "Skipping rating transaction log application"
        return 0
    fi

    log_info "Step 3: Applying rating transaction log to new container"

    # Find the most recent transaction log
    local latest_file=$(ls -t "$BACKUP_DIR"/rating_transactions_*.sql 2>/dev/null | head -n1)

    if [ -z "$latest_file" ]; then
        log_warning "No transaction log files found - this is normal for fresh deployments"
        return 0
    fi

    log_info "Using transaction log: $(basename "$latest_file")"

    if [ "$DRY_RUN" = true ]; then
        log_info "[DRY RUN] Would apply transaction log: $latest_file"
        return 0
    fi

    # Check SQL statement count
    local sql_count=$(grep -v '^--' "$latest_file" 2>/dev/null | grep -v '^$' | wc -l 2>/dev/null || echo "0")

    if [ "$sql_count" -eq 0 ]; then
        log_info "Transaction log is empty - no ratings to apply"
        return 0
    fi

    log_info "Applying $sql_count SQL statements..."

    # Get the new container name
    local new_container=$(docker-compose -f "$DOCKER_COMPOSE_FILE" ps -q | head -n1)
    if [ -z "$new_container" ]; then
        log_error "New container not found"
        return 1
    fi

    local container_name=$(docker inspect --format='{{.Name}}' "$new_container" | sed 's/^.//')

    # Copy transaction log to container
    local container_log_path="/tmp/rating_transactions_apply.sql"
    if docker cp "$latest_file" "$container_name:$container_log_path"; then
        log_info "Transaction log copied to container"
    else
        log_error "Failed to copy transaction log to container"
        return 1
    fi

    # Apply the SQL to the database
    if docker exec "$container_name" sqlite3 "data/zones.db" < "$container_log_path"; then
        log_success "Successfully applied $sql_count rating transactions"

        # Clean up temp file
        docker exec "$container_name" rm -f "$container_log_path"
    else
        log_error "Failed to apply rating transactions"
        return 1
    fi
}

# Verify deployment
verify_deployment() {
    log_info "Step 4: Verifying deployment"

    if [ "$DRY_RUN" = true ]; then
        log_info "[DRY RUN] Would verify deployment"
        return 0
    fi

    # Check if new container is running
    local new_container=$(docker-compose -f "$DOCKER_COMPOSE_FILE" ps -q | head -n1)
    if [ -z "$new_container" ]; then
        log_error "New container is not running"
        return 1
    fi

    local container_name=$(docker inspect --format='{{.Name}}' "$new_container" | sed 's/^.//')
    log_success "New container is running: $container_name"

    # Simple health check - check if we can query the database
    if docker exec "$container_name" sqlite3 "data/zones.db" "SELECT COUNT(*) FROM zones;" > /dev/null 2>&1; then
        log_success "Database is accessible in new container"
    else
        log_warning "Database health check failed"
    fi

    log_success "Deployment verification complete"
}

# Main deployment workflow
main() {
    echo ""
    log_info "========================================="
    log_info "eq_rng.com Deployment with Rating Preservation"
    log_info "========================================="
    echo ""

    log_info "Configuration:"
    echo "  Old container: $OLD_CONTAINER"
    echo "  Backup directory: $BACKUP_DIR"
    echo "  Docker compose: $DOCKER_COMPOSE_FILE"
    echo "  Dry run: $DRY_RUN"
    echo ""

    # Check dependencies
    if ! command -v docker >/dev/null 2>&1; then
        log_error "docker is required but not installed"
        exit 1
    fi

    if ! command -v docker-compose >/dev/null 2>&1; then
        log_error "docker-compose is required but not installed"
        exit 1
    fi

    # Execute deployment steps
    extract_rating_log || {
        log_error "Failed to extract rating transaction log"
        exit 1
    }

    deploy_new_container || {
        log_error "Failed to deploy new container"
        exit 1
    }

    apply_rating_log || {
        log_error "Failed to apply rating transaction log"
        exit 1
    }

    verify_deployment || {
        log_error "Deployment verification failed"
        exit 1
    }

    echo ""
    log_success "========================================="
    log_success "Deployment completed successfully!"
    log_success "User rating data has been preserved"
    log_success "========================================="
    echo ""
}

# Run main function
main "$@"
