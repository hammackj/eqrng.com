#!/bin/bash

# EQ RNG Tests Runner
# This script provides an easy way to run various test suites

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

print_test() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  db        Run database tests"
    echo "  all       Run all test suites"
    echo "  build     Test building all packages"
    echo "  help      Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 db             # Run database connectivity and validation tests"
    echo "  $0 all            # Run all available tests"
    echo "  $0 build          # Test that all packages build successfully"
    echo "  $0 help           # Show this help message"
}

# Function to run database tests
run_database_tests() {
    print_test "Starting database tests..."

    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust first."
        exit 1
    fi

    print_info "Building test binary..."
    cargo build --bin test_db --package eq_rng_tests

    if [ $? -eq 0 ]; then
        print_info "Running database tests..."
        cargo run --bin test_db --package eq_rng_tests

        if [ $? -eq 0 ]; then
            print_info "Database tests completed successfully!"
        else
            print_error "Database tests failed!"
            exit 1
        fi
    else
        print_error "Failed to build test binary!"
        exit 1
    fi
}

# Function to run build tests
run_build_tests() {
    print_test "Starting build tests..."

    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust first."
        exit 1
    fi

    # Test building each package
    packages=("eq_rng" "eq_rng_migrations" "eq_rng_tests")

    for package in "${packages[@]}"; do
        print_info "Building package: $package"
        cargo build --package "$package"

        if [ $? -ne 0 ]; then
            print_error "Failed to build package: $package"
            exit 1
        fi
    done

    # Test building the entire workspace
    print_info "Building entire workspace..."
    cargo build

    if [ $? -eq 0 ]; then
        print_info "All packages built successfully!"
    else
        print_error "Workspace build failed!"
        exit 1
    fi
}

# Function to run all tests
run_all_tests() {
    print_test "Starting comprehensive test suite..."

    # Run build tests first
    run_build_tests

    # Then run database tests
    run_database_tests

    print_info "All test suites completed successfully! 🎉"
}

# Main script logic
case "${1:-}" in
    "db")
        run_database_tests
        ;;
    "build")
        run_build_tests
        ;;
    "all")
        run_all_tests
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
