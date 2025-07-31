#!/bin/bash

# Zone Ratings System Summary and Management Script
# This script provides a comprehensive overview and management interface for the zone ratings system

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BACKUP_DIR="$PROJECT_ROOT/backups/zone_ratings"
MIGRATIONS_DIR="$PROJECT_ROOT/migrations/zone_ratings"
CONTAINER_NAME="${CONTAINER_NAME:-eq_rng-app-1}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Helper functions
log_header() {
    echo -e "${WHITE}$1${NC}"
    echo "=============================================="
}

log_section() {
    echo -e "${CYAN}$1${NC}"
}

log_item() {
    echo -e "${BLUE}•${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

# Check if Docker is running and container exists
check_docker_status() {
    local docker_running=false
    local container_exists=false
    local container_running=false

    if command -v docker >/dev/null 2>&1; then
        if docker info >/dev/null 2>&1; then
            docker_running=true
            if docker ps -a --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
                container_exists=true
                if docker ps --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
                    container_running=true
                fi
            fi
        fi
    fi

    echo "$docker_running,$container_exists,$container_running"
}

# Get zone ratings statistics
get_zone_stats() {
    local use_docker="$1"
    local stats=""

    if [[ "$use_docker" == "true" ]]; then
        stats=$(docker exec "$CONTAINER_NAME" sqlite3 /opt/eq_rng/data/zones.db "
            SELECT
                COUNT(*) as total,
                COUNT(CASE WHEN rating > 0 THEN 1 END) as rated,
                COUNT(CASE WHEN verified = 1 THEN 1 END) as verified,
                ROUND(AVG(CASE WHEN rating > 0 THEN rating END), 1) as avg_rating
            FROM zones;" 2>/dev/null || echo "0|0|0|0")
    else
        if [[ -f "$PROJECT_ROOT/data/zones.db" ]]; then
            stats=$(sqlite3 "$PROJECT_ROOT/data/zones.db" "
                SELECT
                    COUNT(*) as total,
                    COUNT(CASE WHEN rating > 0 THEN 1 END) as rated,
                    COUNT(CASE WHEN verified = 1 THEN 1 END) as verified,
                    ROUND(AVG(CASE WHEN rating > 0 THEN rating END), 1) as avg_rating
                FROM zones;" 2>/dev/null || echo "0|0|0|0")
        else
            echo "0|0|0|0"
        fi
    fi

    echo "$stats"
}

# Count backup files
count_backups() {
    if [[ -d "$BACKUP_DIR" ]]; then
        find "$BACKUP_DIR" -name "zone_ratings_backup_*" -type f | grep -E "\.(json|sql|csv)$" | wc -l | tr -d ' '
    else
        echo "0"
    fi
}

# Count migration scripts
count_migrations() {
    if [[ -d "$MIGRATIONS_DIR" ]]; then
        find "$MIGRATIONS_DIR" -name "auto_migrate_zone_ratings_*.sh" -type f | wc -l | tr -d ' '
    else
        echo "0"
    fi
}

# Get latest backup info
get_latest_backup() {
    if [[ -d "$BACKUP_DIR" ]]; then
        local latest=$(find "$BACKUP_DIR" -name "zone_ratings_backup_*" -type f | grep -E "\.(json|sql|csv)$" | sort | tail -1)
        if [[ -n "$latest" ]]; then
            local size=$(du -h "$latest" 2>/dev/null | cut -f1 || echo "unknown")
            local date=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M" "$latest" 2>/dev/null || stat -c "%y" "$latest" 2>/dev/null | cut -d' ' -f1-2 || echo "unknown")
            echo "$(basename "$latest")|$size|$date"
        else
            echo "none|0|never"
        fi
    else
        echo "none|0|never"
    fi
}

# Display system status
show_system_status() {
    log_header "ZONE RATINGS SYSTEM STATUS"

    # Docker status
    log_section "Docker Environment"
    local docker_status=($(check_docker_status | tr ',' ' '))

    if [[ "${docker_status[0]}" == "true" ]]; then
        log_success "Docker daemon is running"
        if [[ "${docker_status[1]}" == "true" ]]; then
            if [[ "${docker_status[2]}" == "true" ]]; then
                log_success "Container '$CONTAINER_NAME' is running"
            else
                log_warning "Container '$CONTAINER_NAME' exists but is not running"
            fi
        else
            log_warning "Container '$CONTAINER_NAME' does not exist"
        fi
    else
        log_error "Docker daemon is not running"
    fi

    echo ""

    # Database statistics
    log_section "Database Statistics"
    local use_docker="false"
    if [[ "${docker_status[2]}" == "true" ]]; then
        use_docker="true"
    fi

    local stats=($(get_zone_stats "$use_docker" | tr '|' ' '))
    if [[ "${stats[0]}" != "0" || -f "$PROJECT_ROOT/data/zones.db" ]]; then
        log_info "Total zones: ${stats[0]}"
        log_info "Zones with ratings: ${stats[1]}"
        log_info "Verified zones: ${stats[2]}"
        log_info "Average rating: ${stats[3]}"
    else
        log_warning "Database not accessible or empty"
    fi

    echo ""

    # Backup status
    log_section "Backup Status"
    local backup_count=$(count_backups)
    local latest_backup=($(get_latest_backup | tr '|' ' '))

    log_info "Total backups: $backup_count"
    log_info "Latest backup: ${latest_backup[0]}"
    log_info "Backup size: ${latest_backup[1]}"
    log_info "Backup date: ${latest_backup[2]}"

    echo ""

    # Migration status
    log_section "Migration Scripts"
    local migration_count=$(count_migrations)
    log_info "Auto-migration scripts: $migration_count"

    if [[ "$migration_count" -gt 0 ]]; then
        local latest_migration=$(find "$MIGRATIONS_DIR" -name "auto_migrate_zone_ratings_*.sh" -type f | sort | tail -1)
        if [[ -n "$latest_migration" ]]; then
            log_info "Latest migration: $(basename "$latest_migration")"
        fi
    fi

    echo ""
}

# Display available utilities
show_utilities() {
    log_header "AVAILABLE UTILITIES"

    log_section "Backup & Restore"
    log_item "./utils/backup_zone_ratings.sh backup [--docker|--local]"
    log_item "./utils/backup_zone_ratings.sh migrate --file BACKUP_FILE [--docker|--local]"
    log_item "./utils/backup_zone_ratings.sh auto-migrate [--docker|--local]"
    log_item "./utils/backup_zone_ratings.sh status [--docker|--local]"
    log_item "./utils/backup_zone_ratings.sh list-backups"

    echo ""

    log_section "Docker Management"
    log_item "./utils/rebuild_with_zone_ratings.sh                    # Full rebuild with ratings preservation"
    log_item "./utils/rebuild_with_zone_ratings.sh --dry-run          # Preview rebuild process"
    log_item "./utils/rebuild_with_zone_ratings.sh --backup-only      # Only create backup"
    log_item "./utils/rebuild_with_zone_ratings.sh --restore-only FILE # Only restore from backup"

    echo ""

    log_section "Testing & Validation"
    log_item "./utils/test_zone_ratings.sh                           # Run comprehensive test suite"
    log_item "./utils/zone_ratings_summary.sh                       # Show this summary"

    echo ""
}

# Display quick commands
show_quick_commands() {
    log_header "QUICK COMMANDS"

    local docker_status=($(check_docker_status | tr ',' ' '))
    local use_flag="--local"

    if [[ "${docker_status[2]}" == "true" ]]; then
        use_flag="--docker"
        log_section "Using Docker Container"
    else
        log_section "Using Local Database"
    fi

    echo -e "${YELLOW}# Create backup${NC}"
    echo "./utils/backup_zone_ratings.sh backup $use_flag"
    echo ""

    echo -e "${YELLOW}# Check status${NC}"
    echo "./utils/backup_zone_ratings.sh status $use_flag"
    echo ""

    echo -e "${YELLOW}# Create auto-migration script${NC}"
    echo "./utils/backup_zone_ratings.sh auto-migrate $use_flag"
    echo ""

    echo -e "${YELLOW}# Full Docker rebuild with ratings preservation${NC}"
    echo "./utils/rebuild_with_zone_ratings.sh"
    echo ""

    echo -e "${YELLOW}# Run tests${NC}"
    echo "./utils/test_zone_ratings.sh"
    echo ""
}

# Display recent activity
show_recent_activity() {
    log_header "RECENT ACTIVITY"

    # Recent backups
    log_section "Recent Backups (Last 5)"
    if [[ -d "$BACKUP_DIR" ]]; then
        local recent_backups=($(find "$BACKUP_DIR" -name "zone_ratings_backup_*" -type f | grep -E "\.(json|sql|csv)$" | sort | tail -5))

        if [[ ${#recent_backups[@]} -gt 0 ]]; then
            for backup in "${recent_backups[@]}"; do
                local size=$(du -h "$backup" 2>/dev/null | cut -f1 || echo "?")
                local date=$(stat -f "%Sm" -t "%m/%d %H:%M" "$backup" 2>/dev/null || stat -c "%y" "$backup" 2>/dev/null | cut -d' ' -f1-2 || echo "unknown")
                log_item "$(basename "$backup") ($size, $date)"
            done
        else
            log_warning "No backup files found"
        fi
    else
        log_warning "Backup directory does not exist"
    fi

    echo ""

    # Recent migrations
    log_section "Recent Migration Scripts (Last 3)"
    if [[ -d "$MIGRATIONS_DIR" ]]; then
        local recent_migrations=($(find "$MIGRATIONS_DIR" -name "auto_migrate_zone_ratings_*.sh" -type f | sort | tail -3))

        if [[ ${#recent_migrations[@]} -gt 0 ]]; then
            for migration in "${recent_migrations[@]}"; do
                local date=$(stat -f "%Sm" -t "%m/%d %H:%M" "$migration" 2>/dev/null || stat -c "%y" "$migration" 2>/dev/null | cut -d' ' -f1-2 || echo "unknown")
                log_item "$(basename "$migration") ($date)"
            done
        else
            log_warning "No migration scripts found"
        fi
    else
        log_warning "Migrations directory does not exist"
    fi

    echo ""
}

# Display file sizes and disk usage
show_storage_info() {
    log_header "STORAGE INFORMATION"

    log_section "Directory Sizes"

    if [[ -d "$BACKUP_DIR" ]]; then
        local backup_size=$(du -sh "$BACKUP_DIR" 2>/dev/null | cut -f1 || echo "unknown")
        log_info "Backups: $backup_size ($BACKUP_DIR)"
    else
        log_warning "Backup directory does not exist"
    fi

    if [[ -d "$MIGRATIONS_DIR" ]]; then
        local migration_size=$(du -sh "$MIGRATIONS_DIR" 2>/dev/null | cut -f1 || echo "unknown")
        log_info "Migrations: $migration_size ($MIGRATIONS_DIR)"
    else
        log_warning "Migrations directory does not exist"
    fi

    if [[ -f "$PROJECT_ROOT/data/zones.db" ]]; then
        local db_size=$(du -sh "$PROJECT_ROOT/data/zones.db" 2>/dev/null | cut -f1 || echo "unknown")
        log_info "Database: $db_size ($PROJECT_ROOT/data/zones.db)"
    else
        log_warning "Database file not found"
    fi

    echo ""
}

# Main menu
show_help() {
    echo "Zone Ratings System Summary"
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  status      Show system status (default)"
    echo "  utilities   Show available utilities"
    echo "  quick       Show quick commands for current environment"
    echo "  activity    Show recent backup and migration activity"
    echo "  storage     Show storage information"
    echo "  all         Show everything"
    echo "  help        Show this help message"
    echo ""
}

# Main execution
main() {
    local command="${1:-status}"

    case "$command" in
        "status")
            show_system_status
            ;;
        "utilities")
            show_utilities
            ;;
        "quick")
            show_quick_commands
            ;;
        "activity")
            show_recent_activity
            ;;
        "storage")
            show_storage_info
            ;;
        "all")
            show_system_status
            echo ""
            show_quick_commands
            echo ""
            show_recent_activity
            echo ""
            show_storage_info
            echo ""
            show_utilities
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            echo "Unknown command: $command"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
