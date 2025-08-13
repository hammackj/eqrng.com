#!/bin/bash

# Rating Transaction Log Management Utility for eq_rng.com
# Simple utility to extract and apply rating transaction logs from/to Docker containers

set -e  # Exit on any error

# Configuration
CONTAINER_NAME="${CONTAINER_NAME:-eq_rng-app-1}"
LOG_FILE="data/rating_transaction.log"
BACKUP_DIR="${BACKUP_DIR:-backups/rating_transactions}"
DATE=$(date +"%Y%m%d_%H%M%S")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[RATING-LOG]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[RATING-LOG]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[RATING-LOG]${NC} $1"
}

log_error() {
    echo -e "${RED}[RATING-LOG]${NC} $1"
}

print_usage() {
    echo "Rating Transaction Log Management Utility"
    echo "========================================"
    echo ""
    echo "Usage: $0 [OPTIONS] [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  extract             Extract transaction log from Docker container (default)"
    echo "  apply FILE          Apply transaction log file to database"
    echo "  apply-to-docker FILE Apply transaction log to Docker container"
    echo "  show FILE           Display contents of transaction log file"
    echo "  clean              Remove transaction log from container"
    echo "  list               List available transaction log backups"
    echo ""
    echo "Options:"
    echo "  -c, --container NAME    Docker container name (default: $CONTAINER_NAME)"
    echo "  -b, --backup-dir DIR    Backup directory (default: $BACKUP_DIR)"
    echo "  -h, --help             Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 extract                                    # Extract from Docker container"
    echo "  $0 apply rating_transactions_20250101.sql    # Apply to local database"
    echo "  $0 apply-to-docker rating_transactions.sql   # Apply to Docker container"
    echo "  $0 show rating_transactions_20250101.sql     # Show file contents"
    echo ""
    echo "Typical deployment workflow:"
    echo "  1. $0 extract                     # Before stopping old container"
    echo "  2. docker-compose -f docker/docker-compose.yml up --build -d  # Deploy new container"
    echo "  3. $0 apply-to-docker \$latest    # Apply to new container"
}

# Parse command line arguments
COMMAND=""
APPLY_FILE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -c|--container)
            CONTAINER_NAME="$2"
            shift 2
            ;;
        -b|--backup-dir)
            BACKUP_DIR="$2"
            shift 2
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        extract|clean|list)
            COMMAND="$1"
            shift
            ;;
        apply|apply-to-docker|show)
            COMMAND="$1"
            APPLY_FILE="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Default command
if [ -z "$COMMAND" ]; then
    COMMAND="extract"
fi

# Ensure backup directory exists
mkdir -p "$BACKUP_DIR"

# Function to check if container is running
check_container() {
    if ! docker ps --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        log_error "Container '$CONTAINER_NAME' is not running"
        return 1
    fi
    return 0
}

# Extract transaction log from Docker container
extract_from_docker() {
    log_info "Extracting rating transaction log from container: $CONTAINER_NAME"

    if ! check_container; then
        return 1
    fi

    local output_file="$BACKUP_DIR/rating_transactions_$DATE.sql"

    # Check if transaction log exists in container
    if ! docker exec "$CONTAINER_NAME" test -f "$LOG_FILE"; then
        log_warning "No transaction log file found in container"
        log_info "Creating empty transaction log backup: $output_file"
        echo "-- No rating transactions found" > "$output_file"
        return 0
    fi

    # Copy the file from container
    if docker cp "$CONTAINER_NAME:$LOG_FILE" "$output_file"; then
        local line_count=$(wc -l < "$output_file" 2>/dev/null || echo "0")
        log_success "Extracted transaction log to: $output_file"
        log_info "Transaction log contains $line_count lines"

        # Create metadata file
        cat > "$output_file.meta" << EOF
{
    "extract_timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "source_container": "$CONTAINER_NAME",
    "line_count": $line_count,
    "file": "$(basename "$output_file")"
}
EOF
        log_info "Metadata saved to: $output_file.meta"
    else
        log_error "Failed to extract transaction log from container"
        return 1
    fi
}

# Apply transaction log to local database
apply_to_database() {
    local file="$1"

    if [ ! -f "$file" ]; then
        log_error "Transaction log file not found: $file"
        return 1
    fi

    log_info "Applying transaction log to database: $file"

    local db_path="data/zones.db"
    if [ ! -f "$db_path" ]; then
        log_error "Database not found: $db_path"
        return 1
    fi

    # Count SQL statements (excluding comments and empty lines)
    local sql_count=$(grep -v '^--' "$file" | grep -v '^$' | wc -l)
    log_info "Found $sql_count SQL statements to apply"

    if [ "$sql_count" -eq 0 ]; then
        log_warning "No SQL statements to apply"
        return 0
    fi

    # Apply SQL statements
    if sqlite3 "$db_path" < "$file"; then
        log_success "Successfully applied $sql_count SQL statements"
    else
        log_error "Failed to apply transaction log"
        return 1
    fi
}

# Apply transaction log to Docker container
apply_to_docker() {
    local file="$1"

    if [ ! -f "$file" ]; then
        log_error "Transaction log file not found: $file"
        return 1
    fi

    log_info "Applying transaction log to Docker container: $CONTAINER_NAME"

    if ! check_container; then
        return 1
    fi

    # Copy file to container
    local temp_file="/tmp/rating_transactions_apply.sql"
    if docker cp "$file" "$CONTAINER_NAME:$temp_file"; then
        log_info "Transaction log copied to container"
    else
        log_error "Failed to copy transaction log to container"
        return 1
    fi

    # Apply SQL to database inside container
    if docker exec "$CONTAINER_NAME" sqlite3 "$LOG_FILE" < "$temp_file"; then
        local sql_count=$(grep -v '^--' "$file" | grep -v '^$' | wc -l)
        log_success "Successfully applied $sql_count SQL statements to container"

        # Clean up temp file
        docker exec "$CONTAINER_NAME" rm -f "$temp_file"
    else
        log_error "Failed to apply transaction log in container"
        return 1
    fi
}

# Show contents of transaction log file
show_transaction_log() {
    local file="$1"

    if [ ! -f "$file" ]; then
        log_error "Transaction log file not found: $file"
        return 1
    fi

    log_info "Contents of transaction log: $file"
    echo ""

    # Show file with line numbers and syntax highlighting if available
    if command -v bat >/dev/null 2>&1; then
        bat --style=numbers --language=sql "$file"
    elif command -v cat >/dev/null 2>&1; then
        cat -n "$file"
    else
        cat "$file"
    fi

    echo ""
    local line_count=$(wc -l < "$file")
    local sql_count=$(grep -v '^--' "$file" | grep -v '^$' | wc -l)
    log_info "File statistics: $line_count total lines, $sql_count SQL statements"
}

# Clean transaction log from container
clean_transaction_log() {
    log_info "Cleaning transaction log from container: $CONTAINER_NAME"

    if ! check_container; then
        return 1
    fi

    if docker exec "$CONTAINER_NAME" rm -f "$LOG_FILE"; then
        log_success "Transaction log cleaned from container"
    else
        log_warning "Failed to clean transaction log (may not exist)"
    fi
}

# List available transaction log backups
list_backups() {
    log_info "Available transaction log backups in $BACKUP_DIR:"
    echo ""

    if [ -d "$BACKUP_DIR" ] && [ "$(ls -A "$BACKUP_DIR"/*.sql 2>/dev/null)" ]; then
        for file in "$BACKUP_DIR"/*.sql; do
            if [ -f "$file" ]; then
                local size=$(du -h "$file" | cut -f1)
                local line_count=$(wc -l < "$file" 2>/dev/null || echo "0")
                local sql_count=$(grep -v '^--' "$file" 2>/dev/null | grep -v '^$' | wc -l 2>/dev/null || echo "0")

                echo "  $(basename "$file") ($size, $line_count lines, $sql_count SQL statements)"

                # Show metadata if available
                if [ -f "$file.meta" ]; then
                    if command -v jq >/dev/null 2>&1; then
                        local extract_time=$(jq -r '.extract_timestamp' "$file.meta" 2>/dev/null || echo "")
                        local source=$(jq -r '.source_container' "$file.meta" 2>/dev/null || echo "")
                        if [ -n "$extract_time" ] && [ -n "$source" ]; then
                            echo "    └─ Extracted: $extract_time from $source"
                        fi
                    fi
                fi
            fi
        done
    else
        log_warning "No transaction log backups found"
    fi
    echo ""
}

# Main execution
main() {
    case "$COMMAND" in
        extract)
            extract_from_docker
            ;;
        apply)
            if [ -z "$APPLY_FILE" ]; then
                log_error "Apply command requires a file argument"
                print_usage
                exit 1
            fi
            apply_to_database "$APPLY_FILE"
            ;;
        apply-to-docker)
            if [ -z "$APPLY_FILE" ]; then
                log_error "Apply-to-docker command requires a file argument"
                print_usage
                exit 1
            fi
            apply_to_docker "$APPLY_FILE"
            ;;
        show)
            if [ -z "$APPLY_FILE" ]; then
                log_error "Show command requires a file argument"
                print_usage
                exit 1
            fi
            show_transaction_log "$APPLY_FILE"
            ;;
        clean)
            clean_transaction_log
            ;;
        list)
            list_backups
            ;;
        *)
            log_error "Unknown command: $COMMAND"
            print_usage
            exit 1
            ;;
    esac
}

# Show help if no arguments
if [ $# -eq 0 ]; then
    print_usage
    exit 0
fi

# Check dependencies
if ! command -v docker >/dev/null 2>&1; then
    log_error "docker is required but not installed"
    exit 1
fi

# Run main function
main
