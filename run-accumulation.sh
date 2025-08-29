#!/bin/bash

# Load environment variables from .env.docker
set -o allexport
source .env.docker
set +o allexport

# Run docker-compose with the loaded environment
docker-compose -f docker-compose.accumulation.yml up "$@"