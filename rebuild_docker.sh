#!/usr/bin/env bash

export DOCKER_BUILDKIT=1

git pull &&
docker-compose down &&
#docker-compose build --no-cache app &&
docker-compose build app &&
docker-compose up -d &&
docker-compose logs app &&
docker ps


# docker-compose down
# docker-compose rm -f        # remove any stale containers
# docker image prune -f       # remove old images named eqrng_app or eq_rng
# docker-compose build        # build the fresh two-stage image
# docker-compose up -d        # launch it in the background
