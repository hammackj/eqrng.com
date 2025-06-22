#!/usr/bin/env bash

git pull &&
docker-compose down &&
docker-compose build --no-cache app &&
docker-compose up -d &&
docker-compose logs app &&
docker ps
