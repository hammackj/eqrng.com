#!/bin/bash

# Simple Test Script for File-Based Rating Transaction Logging
# Tests that ratings are properly logged to data/rating_transaction.log

set -e  # Exit on any error

# Configuration
API_BASE_URL="${API_BASE_URL:-http://localhost:3000}"
TEST_ZONE_ID="${TEST_ZONE_ID:-1}"
TEST_USER_IP="${TEST_USER_IP:-192.168.1.100}"
LOG_FILE="data/rating_transaction.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# Function to check API health
check_api() {
    if curl -s -f "$API_BASE_URL/version" > /dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Function to submit a rating
submit_rating() {
    local zone_id="$1"
    local rating="$2"
    local user_ip="$3"

    log_info "Submitting rating $rating for zone $zone_id from IP $user_ip"

    local response=$(curl -s -w "%{http_code}" -o /tmp/rating_response.json \
        "$API_BASE_URL/zones/$zone_id/rating?user_ip=$user_ip" \
        -H "Content-Type: application/json" \
        -X POST \
        --data "{\"rating\": $rating}")

    local http_code="${response: -3}"

    if [ "$http_code" = "200" ]; then
        log_success "Rating submitted successfully"
        rm -f /tmp/rating_response.json
        return 0
    else
        log_error "Failed to submit rating (HTTP $http_code)"
        if [ -f /tmp/rating_response.json ]; then
            log_error "Response: $(cat /tmp/rating_response.json)"
            rm -f /tmp/rating_response.json
        fi
        return 1
    fi
}

# Function to check transaction log file
check_transaction_log() {
    log_info "Checking transaction log file: $LOG_FILE"

    if [ ! -f "$LOG_FILE" ]; then
        log_warning "Transaction log file does not exist yet"
        return 1
    fi

    local line_count=$(wc -l < "$LOG_FILE")
    local sql_count=$(grep -v '^--' "$LOG_FILE" | grep -v '^$' | wc -l)

    echo "  File: $LOG_FILE"
    echo "  Lines: $line_count"
    echo "  SQL statements: $sql_count"
    echo ""

    if [ "$sql_count" -gt 0 ]; then
        log_info "Recent transaction log entries:"
        echo ""
        tail -5 "$LOG_FILE" | sed 's/^/  /'
        echo ""
        return 0
    else
        log_warning "No SQL statements found in transaction log"
        return 1
    fi
}

# Function to backup current transaction log
backup_transaction_log() {
    if [ -f "$LOG_FILE" ]; then
        local backup_file="$LOG_FILE.backup_$(date +%Y%m%d_%H%M%S)"
        cp "$LOG_FILE" "$backup_file"
        log_info "Backed up current transaction log to: $backup_file"
    fi
}

# Function to clear transaction log
clear_transaction_log() {
    if [ -f "$LOG_FILE" ]; then
        backup_transaction_log
        > "$LOG_FILE"  # Clear the file
        log_info "Transaction log cleared"
    else
        log_info "No transaction log to clear"
    fi
}

# Function to run complete test
run_complete_test() {
    log_info "Running complete rating transaction log test..."
    echo ""

    # Check initial state
    log_info "=== Initial State ==="
    check_transaction_log || log_info "No existing transaction log (this is normal)"
    echo ""

    # Clear log for clean test
    clear_transaction_log

    # Submit test ratings
    log_info "=== Submitting Test Ratings ==="
    submit_rating "$TEST_ZONE_ID" 3 "$TEST_USER_IP"
    sleep 1

    # Check log after first rating
    check_transaction_log

    # Submit another rating (should be an update)
    submit_rating "$TEST_ZONE_ID" 4 "$TEST_USER_IP"
    sleep 1

    # Submit rating for different zone
    submit_rating "$((TEST_ZONE_ID + 1))" 5 "$TEST_USER_IP"
    echo ""

    # Check final state
    log_info "=== Final Transaction Log ==="
    check_transaction_log

    log_success "Test completed! Check the transaction log file at: $LOG_FILE"
}

# Function to show transaction log contents
show_log_contents() {
    if [ ! -f "$LOG_FILE" ]; then
        log_warning "Transaction log file does not exist: $LOG_FILE"
        return 1
    fi

    log_info "Contents of $LOG_FILE:"
    echo ""

    if command -v bat >/dev/null 2>&1; then
        bat --style=numbers --language=sql "$LOG_FILE"
    else
        cat -n "$LOG_FILE"
    fi

    echo ""
    local line_count=$(wc -l < "$LOG_FILE")
    local sql_count=$(grep -v '^--' "$LOG_FILE" | grep -v '^$' | wc -l)
    log_info "Statistics: $line_count lines, $sql_count SQL statements"
}

# Print usage information
print_usage() {
    echo "Simple Rating Transaction Log Test Script"
    echo "========================================"
    echo ""
    echo "This script tests the file-based rating transaction logging by:"
    echo "1. Submitting test ratings via API"
    echo "2. Checking that SQL statements are written to the log file"
    echo "3. Verifying the log file format"
    echo ""
    echo "Usage: $0 [OPTIONS] [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  test                Run complete test scenario (default)"
    echo "  submit              Submit test ratings only"
    echo "  check               Check transaction log file only"
    echo "  show                Show transaction log contents"
    echo "  clear               Clear transaction log file"
    echo "  backup              Backup current transaction log"
    echo ""
    echo "Options:"
    echo "  -u, --url URL           API base URL (default: $API_BASE_URL)"
    echo "  -z, --zone-id ID        Test zone ID (default: $TEST_ZONE_ID)"
    echo "  -i, --user-ip IP        Test user IP (default: $TEST_USER_IP)"
    echo "  -h, --help              Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                      # Run complete test"
    echo "  $0 submit               # Submit test ratings only"
    echo "  $0 show                 # Show current log contents"
    echo "  $0 clear                # Clear the log file"
}

# Parse command line arguments
COMMAND="test"

while [[ $# -gt 0 ]]; do
    case $1 in
        -u|--url)
            API_BASE_URL="$2"
            shift 2
            ;;
        -z|--zone-id)
            TEST_ZONE_ID="$2"
            shift 2
            ;;
        -i|--user-ip)
            TEST_USER_IP="$2"
            shift 2
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        test|submit|check|show|clear|backup)
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
    log_info "Rating Transaction Log Test Script"
    echo ""

    # Check dependencies
    if ! command -v curl >/dev/null 2>&1; then
        log_error "curl is required but not installed"
        exit 1
    fi

    case "$COMMAND" in
        test)
            # Check API availability for test command
            if ! check_api; then
                log_error "API is not available at $API_BASE_URL"
                log_error "Make sure the eq_rng application is running"
                exit 1
            fi
            log_success "API is available at $API_BASE_URL"
            echo ""
            run_complete_test
            ;;
        submit)
            if ! check_api; then
                log_error "API is not available at $API_BASE_URL"
                exit 1
            fi
            log_info "=== Submitting Test Ratings ==="
            submit_rating "$TEST_ZONE_ID" 3 "$TEST_USER_IP"
            submit_rating "$TEST_ZONE_ID" 4 "$TEST_USER_IP"
            submit_rating "$((TEST_ZONE_ID + 1))" 5 "$TEST_USER_IP"
            log_success "Test ratings submitted"
            ;;
        check)
            check_transaction_log
            ;;
        show)
            show_log_contents
            ;;
        clear)
            clear_transaction_log
            ;;
        backup)
            backup_transaction_log
            ;;
        *)
            log_error "Unknown command: $COMMAND"
            print_usage
            exit 1
            ;;
    esac
}

# Run main function
main
```

Let me make it executable and remove the old complex scripts:
