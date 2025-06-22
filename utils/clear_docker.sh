#!/usr/bin/env bash
set -euo pipefail

# Stop all running containers
if [ -n "$(docker ps -q)" ]; then
  echo "Stopping all containers..."
  docker ps -q | xargs -r docker stop
else
  echo "No running containers."
fi

# Remove all containers
if [ -n "$(docker ps -aq)" ]; then
  echo "Removing all containers..."
  docker ps -aq | xargs -r docker rm
else
  echo "No containers to remove."
fi

# Remove all images
if [ -n "$(docker images -q)" ]; then
  echo "Removing all images..."
  docker images -q | xargs -r docker rmi -f
else
  echo "No images to remove."
fi

echo "Done. All Docker images (and containers) have been cleared."
