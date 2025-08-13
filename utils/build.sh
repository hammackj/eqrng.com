#!/bin/bash

# Build script for EQ RNG application
# Usage: ./build.sh [production|development|local]
#
# Note: This script is in the utils/ directory but runs Docker commands from
# the project root directory to access all necessary files.

set -euo pipefail

ENV=${1:-production}

# Change to project root directory
cd "$(dirname "$0")/.."

echo "Building EQ RNG for environment: $ENV"

case $ENV in
    "production")
        echo "Building production image (no admin interface)..."
        # Dockerfile now lives at docker/Dockerfile
        docker build -t eq_rng:latest -f docker/Dockerfile .
        echo "✅ Production build complete! Image tagged as eq_rng:latest"
        echo "To run: docker-compose -f docker/docker-compose.yml up"
        ;;

    "development")
        echo "Building development image (with admin interface)..."
        # Dockerfile.dev now lives at docker/Dockerfile.dev
        docker build -t eq_rng:dev -f docker/Dockerfile.dev .
        echo "✅ Development build complete! Image tagged as eq_rng:dev"
        echo "To run: docker-compose -f docker/docker-compose.dev.yml up"
        ;;

    "local")
        echo "Building locally with admin features..."
        cd frontend && npm install && npm run build && cd ..
        cargo build --release --features admin
        echo "✅ Local build complete!"
        echo "To run locally: ./target/release/eq_rng"
        ;;

    *)
        echo "❌ Invalid environment. Use: production, development, or local"
        echo "Examples:"
        echo "  ./utils/build.sh production   # Build production Docker image (no admin)"
        echo "  ./utils/build.sh development  # Build development Docker image (with admin)"
        echo "  ./utils/build.sh local        # Build locally with admin features"
        exit 1
        ;;
esac

echo ""
echo "Admin interface availability:"
echo "  Production:  ❌ Disabled"
echo "  Development: ✅ Enabled at /admin"
echo "  Local:       ✅ Enabled at /admin"
