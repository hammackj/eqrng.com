# EQRng.com

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
