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

echo -e "${BLUE}Building all Fossil images...${NC}"

# Build network images
echo -e "${BLUE}Step 1/2: Building network images...${NC}"
./scripts/build-network.sh $VERBOSE
echo -e "${GREEN}Network images built successfully!${NC}"

# Build service images
echo -e "${BLUE}Step 2/2: Building service images...${NC}"
./scripts/build-services.sh $VERBOSE
echo -e "${GREEN}Service images built successfully!${NC}"

echo -e "${GREEN}All Fossil images built successfully!${NC}" 