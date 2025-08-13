#!/usr/bin/env bash

export DOCKER_BUILDKIT=1

git pull &&
docker-compose -f docker/docker-compose.yml down &&
#docker-compose build --no-cache app &&
docker-compose -f docker/docker-compose.yml build app &&
docker-compose -f docker/docker-compose.yml up -d &&
docker-compose -f docker/docker-compose.yml logs app &&
docker ps


# docker-compose -f docker/docker-compose.yml down
# docker-compose -f docker/docker-compose.yml rm -f        # remove any stale containers
# docker image prune -f       # remove old images named eqrng_app or eq_rng
# docker-compose -f docker/docker-compose.yml build        # build the fresh two-stage image
# docker-compose -f docker/docker-compose.yml up -d        # launch it in the background
