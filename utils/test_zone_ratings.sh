#!/bin/bash

# Test Script for Zone Ratings Backup and Migration System
# This script tests the zone ratings backup, migration, and restoration functionality

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_DIR="$PROJECT_ROOT/test_zone_ratings"
TEST_DB="$TEST_DIR/test_zones.db"
TEST_BACKUP_DIR="$TEST_DIR/backups"
TEST_MIGRATIONS_DIR="$TEST_DIR/migrations"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

# Helper functions
log_info() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[TEST]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[TEST]${NC} $1"
}

log_error() {
    echo -e "${RED}[TEST]${NC} $1"
}

log_test_header() {
    echo ""
    echo -e "${CYAN}[TEST]${NC} =================================="
    echo -e "${CYAN}[TEST]${NC} $1"
    echo -e "${CYAN}[TEST]${NC} =================================="
}

# Test framework functions
start_test() {
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    log_info "Starting test: $1"
}

pass_test() {
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_success "✓ PASSED: $1"
}

fail_test() {
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ FAILED: $1"
    if [[ -n "$2" ]]; then
        log_error "  Reason: $2"
    fi
}

assert_file_exists() {
    if [[ -f "$1" ]]; then
        pass_test "File exists: $(basename "$1")"
    else
        fail_test "File does not exist: $1"
        return 1
    fi
}

assert_file_not_empty() {
    if [[ -f "$1" && -s "$1" ]]; then
        pass_test "File is not empty: $(basename "$1")"
    else
        fail_test "File is empty or does not exist: $1"
        return 1
    fi
}

assert_command_success() {
    local command="$1"
    local description="$2"

    if eval "$command" >/dev/null 2>&1; then
        pass_test "$description"
    else
        fail_test "$description" "Command failed: $command"
        return 1
    fi
}

assert_db_query() {
    local query="$1"
    local expected="$2"
    local description="$3"

    local result=$(sqlite3 "$TEST_DB" "$query" 2>/dev/null || echo "ERROR")

    if [[ "$result" == "$expected" ]]; then
        pass_test "$description"
    else
        fail_test "$description" "Expected '$expected', got '$result'"
        return 1
    fi
}

# Setup functions
setup_test_environment() {
    log_test_header "SETTING UP TEST ENVIRONMENT"

    # Clean up any existing test environment
    rm -rf "$TEST_DIR" 2>/dev/null || true

    # Create test directories
    mkdir -p "$TEST_DIR"
    mkdir -p "$TEST_BACKUP_DIR"
    mkdir -p "$TEST_MIGRATIONS_DIR"

    log_success "Test environment created: $TEST_DIR"
}

create_test_database() {
    log_test_header "CREATING TEST DATABASE"

    # Create a minimal zones table
    sqlite3 "$TEST_DB" <<EOF
CREATE TABLE zones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    level_ranges TEXT NOT NULL,
    expansion TEXT NOT NULL,
    continent TEXT NOT NULL DEFAULT '',
    zone_type TEXT NOT NULL,
    connections TEXT NOT NULL DEFAULT '[]',
    image_url TEXT NOT NULL DEFAULT '',
    map_url TEXT NOT NULL DEFAULT '',
    rating INTEGER NOT NULL DEFAULT 0,
    hot_zone BOOLEAN NOT NULL DEFAULT FALSE,
    mission BOOLEAN NOT NULL DEFAULT FALSE,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Insert test zones with various ratings
INSERT INTO zones (name, level_ranges, expansion, zone_type, rating, verified) VALUES
('test_zone_1', '[[1,10]]', 'Classic', 'outdoor', 5, 1),
('test_zone_2', '[[10,20]]', 'Classic', 'outdoor', 4, 1),
('test_zone_3', '[[20,30]]', 'Kunark', 'outdoor', 3, 0),
('test_zone_4', '[[30,40]]', 'Kunark', 'outdoor', 0, 0),
('test_zone_5', '[[40,50]]', 'Velious', 'outdoor', 2, 1),
('test_zone_unrated', '[[50,60]]', 'Velious', 'outdoor', 0, 0);
EOF

    assert_file_exists "$TEST_DB"
    assert_db_query "SELECT COUNT(*) FROM zones;" "6" "Database has 6 test zones"
    assert_db_query "SELECT COUNT(*) FROM zones WHERE rating > 0;" "4" "Database has 4 rated zones"
}

# Test functions
test_backup_script_exists() {
    log_test_header "TESTING BACKUP SCRIPT AVAILABILITY"

    start_test "Backup script exists and is executable"
    assert_file_exists "$SCRIPT_DIR/backup_zone_ratings.sh"
    assert_command_success "test -x '$SCRIPT_DIR/backup_zone_ratings.sh'" "Backup script is executable"
}

test_backup_help() {
    log_test_header "TESTING BACKUP SCRIPT HELP"

    start_test "Backup script shows help"
    assert_command_success "'$SCRIPT_DIR/backup_zone_ratings.sh' --help" "Help command works"
}

test_json_backup() {
    log_test_header "TESTING JSON BACKUP"

    # Override paths for testing
    export DB_PATH="$TEST_DB"
    export BACKUP_DIR="$TEST_BACKUP_DIR"

    start_test "Create JSON backup"
    local backup_output=$("$SCRIPT_DIR/backup_zone_ratings.sh" backup --format json --local 2>&1)
    local backup_file=$(echo "$backup_output" | grep -E "\.json$" | head -1 | sed 's/.*: //')

    log_info "Backup file path: $backup_file"

    if [[ -n "$backup_file" && -f "$backup_file" ]]; then
        pass_test "JSON backup created: $(basename "$backup_file")"

        # Test backup content
        start_test "JSON backup content validation"
        if command -v jq >/dev/null 2>&1; then
            local zone_count=$(jq '. | length' "$backup_file" 2>/dev/null || echo "0")
            if [[ "$zone_count" == "4" ]]; then
                pass_test "JSON backup contains correct number of zones"
            else
                fail_test "JSON backup contains wrong number of zones" "Expected 4, got $zone_count"
            fi

            # Test specific zone data
            local test_zone_rating=$(jq -r '.[] | select(.name=="test_zone_1") | .rating' "$backup_file" 2>/dev/null || echo "null")
            if [[ "$test_zone_rating" == "5" ]]; then
                pass_test "JSON backup contains correct zone rating"
            else
                fail_test "JSON backup has incorrect zone rating" "Expected 5, got $test_zone_rating"
            fi
        else
            log_warning "jq not available, skipping JSON content validation"
        fi

        # Save for restoration test
        export TEST_JSON_BACKUP="$backup_file"
    else
        fail_test "JSON backup creation failed"
    fi
}

test_sql_backup() {
    log_test_header "TESTING SQL BACKUP"

    start_test "Create SQL backup"
    local backup_output=$("$SCRIPT_DIR/backup_zone_ratings.sh" backup --format sql --local 2>&1)
    local backup_file=$(echo "$backup_output" | grep -E "\.sql$" | head -1 | sed 's/.*: //')

    if [[ -n "$backup_file" && -f "$backup_file" ]]; then
        pass_test "SQL backup created: $(basename "$backup_file")"

        # Test backup content
        start_test "SQL backup content validation"
        local update_count=$(grep -c "UPDATE zones SET" "$backup_file" 2>/dev/null || echo "0")
        if [[ "$update_count" == "4" ]]; then
            pass_test "SQL backup contains correct number of update statements"
        else
            fail_test "SQL backup contains wrong number of updates" "Expected 4, got $update_count"
        fi

        # Save for restoration test
        export TEST_SQL_BACKUP="$backup_file"
    else
        fail_test "SQL backup creation failed"
    fi
}

test_csv_backup() {
    log_test_header "TESTING CSV BACKUP"

    start_test "Create CSV backup"
    local backup_output=$("$SCRIPT_DIR/backup_zone_ratings.sh" backup --format csv --local 2>&1)
    local backup_file=$(echo "$backup_output" | grep -E "\.csv$" | head -1 | sed 's/.*: //')

    if [[ -n "$backup_file" && -f "$backup_file" ]]; then
        pass_test "CSV backup created: $(basename "$backup_file")"

        # Test backup content
        start_test "CSV backup content validation"
        local line_count=$(wc -l < "$backup_file" 2>/dev/null | tr -d ' ' || echo "0")
        if [[ "$line_count" == "5" ]]; then  # Header + 4 data lines
            pass_test "CSV backup contains correct number of lines"
        else
            fail_test "CSV backup contains wrong number of lines" "Expected 5, got $line_count"
        fi

        # Save for restoration test
        export TEST_CSV_BACKUP="$backup_file"
    else
        fail_test "CSV backup creation failed"
    fi
}

test_backup_with_min_rating() {
    log_test_header "TESTING BACKUP WITH MINIMUM RATING FILTER"

    start_test "Create backup with min-rating filter"
    local backup_output=$("$SCRIPT_DIR/backup_zone_ratings.sh" backup --format json --min-rating 4 --local 2>&1)
    local backup_file=$(echo "$backup_output" | grep -E "\.json$" | head -1 | sed 's/.*: //')

    if [[ -n "$backup_file" && -f "$backup_file" ]] && command -v jq >/dev/null 2>&1; then
        local zone_count=$(jq '. | length' "$backup_file" 2>/dev/null || echo "0")
        if [[ "$zone_count" == "2" ]]; then  # Only zones with rating >= 4
            pass_test "Min-rating filter works correctly"
        else
            fail_test "Min-rating filter incorrect" "Expected 2 zones with rating >= 4, got $zone_count"
        fi
    else
        log_warning "Cannot test min-rating filter (backup failed or jq not available)"
    fi
}

test_zone_modification_and_restore() {
    log_test_header "TESTING ZONE MODIFICATION AND RESTORE"

    # Modify some zone ratings
    start_test "Modify zone ratings"
    sqlite3 "$TEST_DB" "UPDATE zones SET rating = 1 WHERE name = 'test_zone_1';"
    sqlite3 "$TEST_DB" "UPDATE zones SET rating = 0 WHERE name = 'test_zone_2';"

    assert_db_query "SELECT rating FROM zones WHERE name = 'test_zone_1';" "1" "Zone 1 rating modified"
    assert_db_query "SELECT rating FROM zones WHERE name = 'test_zone_2';" "0" "Zone 2 rating modified"

    # Restore from JSON backup
    if [[ -n "$TEST_JSON_BACKUP" && -f "$TEST_JSON_BACKUP" ]]; then
        start_test "Restore from JSON backup"
        "$SCRIPT_DIR/backup_zone_ratings.sh" migrate --file "$TEST_JSON_BACKUP" --local --force

        assert_db_query "SELECT rating FROM zones WHERE name = 'test_zone_1';" "5" "Zone 1 rating restored from JSON"
        assert_db_query "SELECT rating FROM zones WHERE name = 'test_zone_2';" "4" "Zone 2 rating restored from JSON"
    else
        log_warning "Skipping JSON restore test (no backup file)"
    fi
}

test_auto_migration_generation() {
    log_test_header "TESTING AUTO-MIGRATION GENERATION"

    export MIGRATIONS_DIR="$TEST_MIGRATIONS_DIR"

    start_test "Generate auto-migration script"
    local migration_output=$("$SCRIPT_DIR/backup_zone_ratings.sh" auto-migrate --local 2>&1)
    local migration_script=$(echo "$migration_output" | grep -E "\.sh$" | head -1 | sed 's/.*: //')

    log_info "Migration script path: $migration_script"

    if [[ -n "$migration_script" && -f "$migration_script" ]]; then
        pass_test "Auto-migration script created: $(basename "$migration_script")"

        start_test "Migration script is executable"
        if [[ -x "$migration_script" ]]; then
            pass_test "Migration script is executable"
        else
            fail_test "Migration script is not executable"
        fi

        start_test "Migration script contains expected content"
        if grep -q "Zone Ratings Migration Script" "$migration_script"; then
            pass_test "Migration script contains header"
        else
            fail_test "Migration script missing expected header"
        fi

        export TEST_MIGRATION_SCRIPT="$migration_script"
    else
        fail_test "Auto-migration script generation failed"
    fi
}

test_migration_script_execution() {
    log_test_header "TESTING MIGRATION SCRIPT EXECUTION"

    if [[ -n "$TEST_MIGRATION_SCRIPT" && -f "$TEST_MIGRATION_SCRIPT" && -n "$TEST_JSON_BACKUP" && -f "$TEST_JSON_BACKUP" ]]; then
        # Modify ratings again
        sqlite3 "$TEST_DB" "UPDATE zones SET rating = 0 WHERE rating > 0;"
        assert_db_query "SELECT COUNT(*) FROM zones WHERE rating > 0;" "0" "All ratings reset to 0"

        # Set environment for migration script and override backup file path
        export DB_PATH="$TEST_DB"

        # Modify the migration script to use our test backup file
        local temp_script="/tmp/test_migration_$$.sh"
        sed "s|BACKUP_FILE=.*|BACKUP_FILE=\"$TEST_JSON_BACKUP\"|" "$TEST_MIGRATION_SCRIPT" > "$temp_script"
        chmod +x "$temp_script"

        start_test "Execute migration script"
        if "$temp_script" >/dev/null 2>&1; then
            pass_test "Migration script executed successfully"

            # Check if ratings were restored
            local rated_count=$(sqlite3 "$TEST_DB" "SELECT COUNT(*) FROM zones WHERE rating > 0;" 2>/dev/null || echo "0")
            if [[ "$rated_count" -gt "0" ]]; then
                pass_test "Migration script restored zone ratings"
            else
                fail_test "Migration script did not restore zone ratings"
            fi
        else
            fail_test "Migration script execution failed"
        fi

        # Clean up temp script
        rm -f "$temp_script"
    else
        log_warning "Skipping migration script execution test (no script or backup file available)"
    fi
}

test_status_command() {
    log_test_header "TESTING STATUS COMMAND"

    start_test "Status command execution"
    if "$SCRIPT_DIR/backup_zone_ratings.sh" status --local >/dev/null 2>&1; then
        pass_test "Status command executed successfully"
    else
        fail_test "Status command failed"
    fi
}

test_list_backups_command() {
    log_test_header "TESTING LIST-BACKUPS COMMAND"

    start_test "List-backups command execution"
    if "$SCRIPT_DIR/backup_zone_ratings.sh" list-backups >/dev/null 2>&1; then
        pass_test "List-backups command executed successfully"
    else
        fail_test "List-backups command failed"
    fi
}

# Cleanup function
cleanup_test_environment() {
    log_test_header "CLEANING UP TEST ENVIRONMENT"

    # Restore original environment
    unset DB_PATH BACKUP_DIR MIGRATIONS_DIR

    # Clean up test files
    if [[ -d "$TEST_DIR" ]]; then
        rm -rf "$TEST_DIR"
        log_success "Test environment cleaned up"
    fi
}

# Results summary
print_test_results() {
    echo ""
    echo "=============================================="
    echo "TEST RESULTS SUMMARY"
    echo "=============================================="
    echo "Total tests: $TESTS_TOTAL"
    echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Failed: ${RED}$TESTS_FAILED${NC}"

    if [[ $TESTS_FAILED -eq 0 ]]; then
        echo -e "${GREEN}ALL TESTS PASSED!${NC}"
        return 0
    else
        echo -e "${RED}SOME TESTS FAILED!${NC}"
        return 1
    fi
}

# Main test execution
main() {
    echo "=============================================="
    echo "Zone Ratings Backup System Test Suite"
    echo "=============================================="
    echo "Started at: $(date)"
    echo ""

    # Check prerequisites
    if ! command -v sqlite3 >/dev/null 2>&1; then
        log_error "sqlite3 is required for testing"
        exit 1
    fi

    # Run tests
    setup_test_environment
    create_test_database
    test_backup_script_exists
    test_backup_help
    test_json_backup
    test_sql_backup
    test_csv_backup
    test_backup_with_min_rating
    test_zone_modification_and_restore
    test_auto_migration_generation
    test_migration_script_execution
    test_status_command
    test_list_backups_command

    # Cleanup and results
    cleanup_test_environment
    print_test_results
}

# Handle cleanup on script exit
trap cleanup_test_environment EXIT

# Run tests
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
