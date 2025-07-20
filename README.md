# EQRng.com

Server runs on port 3000 this is exported from docker to the host machine. The host machine is running ngix and proxies 3000 to 80/443. Currently the frontend is a dev test front end. I plan on making a proper frontend once I have all the features implemented.

## /random_zone
### Parameters
- min_level
- max_level
- zone_type

This endpoint will return a random zone based on the given parameters.

# Deployment

  cd /home/deploy/eq_rng
  docker-compose up -d --build
  curl http://localhost:8000/random_zone
