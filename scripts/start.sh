#!/bin/bash
set -e

ENV_FILE=$1
BUILD=$2  # Optional second parameter to trigger build

if [ "$ENV_FILE" != ".env.local" ] && [ "$ENV_FILE" != ".env.sepolia" ] && [ "$ENV_FILE" != ".env.mainnet" ]; then
  echo "Usage: $0 { .env.local | .env.sepolia | .env.mainnet } [build]"
  exit 1
fi

export ENV_FILE=$ENV_FILE

# Clean up any existing containers
docker-compose down

if [ "$BUILD" == "build" ]; then
  # Build images
  echo "Building images..."
  docker-compose -f docker-compose.yml -f docker-compose.build.yml build
fi

if [ "$ENV_FILE" == ".env.local" ]; then
  # Local development setup
  docker-compose up -d
else
  # Production setup (sepolia/mainnet)
  docker-compose up -d client relayer
fi
