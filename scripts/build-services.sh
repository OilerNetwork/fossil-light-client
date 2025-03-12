#!/bin/bash

set -e

# Define colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# Parse command line arguments
VERBOSE=""
if [[ " $* " =~ " --verbose " ]] || [[ " $* " =~ " -v " ]]; then
    VERBOSE="--verbose"
fi

echo -e "${BLUE}Building Fossil service images...${NC}"

# Build MMR image
echo -e "${BLUE}Step 1/3: Building MMR image...${NC}"
./scripts/build-mmr-image.sh
echo -e "${GREEN}MMR image built successfully!${NC}"

# Build client image
echo -e "${BLUE}Step 2/3: Building client image...${NC}"
./scripts/build-client-image.sh
echo -e "${GREEN}Client image built successfully!${NC}"

# Build relayer image
echo -e "${BLUE}Step 3/3: Building relayer image...${NC}"
# Ensure we're using buildx
docker buildx create --use --name fossil-builder || true

docker buildx build --load -f docker/Dockerfile.relayer -t fossil-relayer:latest . $([[ -n "$VERBOSE" ]] && echo "--progress=plain")

# Clean up the builder
docker buildx rm fossil-builder

echo -e "${GREEN}Relayer image built successfully!${NC}"

echo -e "${GREEN}All service images built successfully!${NC}" 