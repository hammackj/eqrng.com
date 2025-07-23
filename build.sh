#!/bin/bash

# Build script for EQ RNG application
# Usage: ./build.sh [production|development|local]

set -e

ENV=${1:-production}

echo "Building EQ RNG for environment: $ENV"

case $ENV in
    "production")
        echo "Building production image (no admin interface)..."
        docker build -t eq_rng:latest -f Dockerfile .
        echo "✅ Production build complete! Image tagged as eq_rng:latest"
        echo "To run: docker-compose up"
        ;;

    "development")
        echo "Building development image (with admin interface)..."
        docker build -t eq_rng:dev -f Dockerfile.dev .
        echo "✅ Development build complete! Image tagged as eq_rng:dev"
        echo "To run: docker-compose -f docker-compose.dev.yml up"
        ;;

    "local")
        echo "Building locally with admin features..."
        cd frontend && npm install && npm run build && cd ..
        cargo build --release --features admin
        echo "✅ Local build complete!"
        echo "To run: ./target/release/eq_rng"
        ;;

    *)
        echo "❌ Invalid environment. Use: production, development, or local"
        echo "Examples:"
        echo "  ./build.sh production   # Build production Docker image (no admin)"
        echo "  ./build.sh development  # Build development Docker image (with admin)"
        echo "  ./build.sh local        # Build locally with admin features"
        exit 1
        ;;
esac

echo ""
echo "Admin interface availability:"
echo "  Production:  ❌ Disabled"
echo "  Development: ✅ Enabled at /admin"
echo "  Local:       ✅ Enabled at /admin"
