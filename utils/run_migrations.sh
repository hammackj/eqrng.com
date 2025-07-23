#!/bin/bash

# EQ RNG Migrations Runner
# This script provides an easy way to run various migration tasks

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  zones     Run zone data migration"
    echo "  help      Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 zones           # Migrate zone data from JSON to database"
    echo "  $0 help            # Show this help message"
}

# Function to run zone migration
run_zone_migration() {
    print_info "Starting zone data migration..."

    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust first."
        exit 1
    fi

    print_info "Building migration binary..."
    cargo build --bin migrate_zones --package eq_rng_migrations --release

    if [ $? -eq 0 ]; then
        print_info "Running zone migration..."
        cargo run --bin migrate_zones --package eq_rng_migrations --release

        if [ $? -eq 0 ]; then
            print_info "Zone migration completed successfully!"
        else
            print_error "Zone migration failed!"
            exit 1
        fi
    else
        print_error "Failed to build migration binary!"
        exit 1
    fi
}

# Main script logic
case "${1:-}" in
    "zones")
        run_zone_migration
        ;;
    "help"|"-h"|"--help")
        show_usage
        ;;
    "")
        print_warning "No command specified."
        echo ""
        show_usage
        exit 1
        ;;
    *)
        print_error "Unknown command: $1"
        echo ""
        show_usage
        exit 1
        ;;
esac
