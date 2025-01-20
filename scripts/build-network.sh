#!/bin/bash

set -e

# Parse command line arguments
VERBOSE=""
if [[ " $* " =~ " --verbose " ]] || [[ " $* " =~ " -v " ]]; then
    VERBOSE="--progress=plain"
fi

# Define colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# Detect platform
PLATFORM="linux/amd64"
if [[ "$(uname -m)" == "arm64" && "$(uname -s)" == "Darwin" ]]; then
    echo -e "${BLUE}Detected Apple Silicon (M1/M2) - Using platform flag${NC}"
    PLATFORM_FLAG="--platform linux/amd64"
else
    echo -e "${BLUE}Detected Linux/AMD64 - Using default platform${NC}"
    PLATFORM_FLAG=""
fi

# Ensure we're using buildx
docker buildx create --use --name fossil-builder || true

echo -e "${BLUE}Building network Docker images...${NC}"

# Build network images
echo -e "${BLUE}Building anvil image...${NC}"
docker buildx build --load $PLATFORM_FLAG -f docker/Dockerfile.anvil -t fossil-anvil:latest . $VERBOSE

echo -e "${BLUE}Building katana image...${NC}"
docker buildx build --load $PLATFORM_FLAG -f docker/Dockerfile.katana -t fossil-katana:latest . $VERBOSE

echo -e "${BLUE}Building deploy image...${NC}"
docker buildx build --load $PLATFORM_FLAG -f docker/Dockerfile.deploy -t fossil-deploy:latest . $VERBOSE

# Clean up the builder
echo -e "${BLUE}Cleaning up builder...${NC}"
docker buildx rm fossil-builder

echo -e "${GREEN}Network images built successfully!${NC}" 