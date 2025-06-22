#!/usr/bin/env bash

export DOCKER_BUILDKIT=1

git pull &&
docker-compose down &&
docker-compose build --no-cache app &&
docker-compose up -d &&
docker-compose logs app &&
docker ps
