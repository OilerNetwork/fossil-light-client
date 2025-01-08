#!/bin/bash

# Ensure the script stops on the first error
set -e

# Check if environment argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <environment>"
    echo "Available environments: local, sepolia, mainnet"
    exit 1
fi

# Validate environment argument
ENV_TYPE="$1"
case "$ENV_TYPE" in
"local" | "sepolia" | "mainnet" | "docker")
    ENV_FILE=".env.$ENV_TYPE"
    # Add support for updating both files in docker mode
    if [ "$ENV_TYPE" = "docker" ]; then
        ENV_FILES=(".env.docker" ".env.local")
        echo "Using environment: $ENV_TYPE (updating both ${ENV_FILES[*]})"
    else
        ENV_FILES=("$ENV_FILE")
        echo "Using environment: $ENV_TYPE ($ENV_FILE)"
    fi
    ;;
*)
    echo "Invalid environment. Must be one of: local, sepolia, mainnet"
    exit 1
    ;;
esac

# Source the primary environment file
source "${ENV_FILES[0]}"
export ACCOUNT_PRIVATE_KEY=${ACCOUNT_PRIVATE_KEY}

# Use relative paths instead of absolute Docker paths
ETHEREUM_DIR="contracts/ethereum"
CONFIG_DIR="config"

# Define colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'
BOLD='\033[1m'
RED='\033[0;31m'

# Function to update environment variables
update_env_var() {
    local env_file=$1
    local var_name=$2
    local var_value=$3

    if grep -q "^$var_name=" "$env_file"; then
        echo -e "${BLUE}$var_name exists, updating...${NC}"
        # Create a temporary file in case sed -i doesn't work on your system
        sed "s|^$var_name=.*|$var_name=$var_value|" "$env_file" > "${env_file}.tmp"
        mv "${env_file}.tmp" "$env_file"
    else
        echo -e "${BLUE}$var_name not found, appending...${NC}"
        echo "$var_name=$var_value" >> "$env_file"
    fi
}

# Function to update JSON config
update_json_config() {
    local json_file=$1
    local contract_address=$2
    
    # Create temp file in the same directory to avoid permission issues
    local tmp_file="${json_file}.tmp"
    
    if ! jq --arg addr "$contract_address" '.contract_address = $addr' "$json_file" > "$tmp_file"; then
        echo -e "${RED}Failed to update JSON file${NC}"
        rm -f "$tmp_file"
        return 1
    fi
    
    if ! mv "$tmp_file" "$json_file"; then
        echo -e "${RED}Failed to replace JSON file${NC}"
        rm -f "$tmp_file"
        return 1
    fi
    
    echo -e "${BLUE}Updated contract address in $json_file${NC}"
}

# Function to deploy with retries
deploy_contracts() {
    local max_attempts=3
    local attempt=1
    local wait_time=10

    while [ $attempt -le $max_attempts ]; do
        echo -e "${BLUE}${BOLD}Deploying Ethereum contracts (Attempt $attempt/$max_attempts)...${NC}"
        
        if forge script script/LocalTesting.s.sol:LocalSetup --broadcast --rpc-url $ETH_RPC_URL; then
            return 0
        fi
        
        if [ $attempt -lt $max_attempts ]; then
            echo -e "${YELLOW}Deployment failed, retrying in ${wait_time}s...${NC}"
            sleep $wait_time
        fi
        
        attempt=$((attempt + 1))
    done

    echo -e "${RED}Failed to deploy contracts after $max_attempts attempts${NC}"
    return 1
}

# Store the root directory path
ROOT_DIR=$(pwd)

# Deploy Ethereum contracts
cd "$ETHEREUM_DIR"
deploy_contracts || exit 1

# Add debug logging
echo -e "${YELLOW}Current directory: $(pwd)${NC}"
echo -e "${YELLOW}Looking for file: logs/local_setup.json${NC}"

# Read values from the JSON file and update env vars
if [ -f "logs/local_setup.json" ]; then
    echo -e "${YELLOW}Found local_setup.json${NC}"
    
    SN_MESSAGING=$(jq -r '.snMessaging_address' logs/local_setup.json)
    L1_MESSAGE_SENDER=$(jq -r '.l1MessageSender_address' logs/local_setup.json)
    
    echo -e "${YELLOW}Read values:${NC}"
    echo -e "${YELLOW}SN_MESSAGING: $SN_MESSAGING${NC}"
    echo -e "${YELLOW}L1_MESSAGE_SENDER: $L1_MESSAGE_SENDER${NC}"
    
    # Update the environment variables - use full paths
    for env_file in "${ENV_FILES[@]}"; do
        update_env_var "${ROOT_DIR}/${env_file}" "SN_MESSAGING" "$SN_MESSAGING"
        update_env_var "${ROOT_DIR}/${env_file}" "L1_MESSAGE_SENDER" "$L1_MESSAGE_SENDER"
        
        # Verify the updates
        echo -e "${YELLOW}Checking updated ${env_file}:${NC}"
        grep "SN_MESSAGING" "${ROOT_DIR}/${env_file}"
        grep "L1_MESSAGE_SENDER" "${ROOT_DIR}/${env_file}"
    done
else
    echo -e "${RED}Could not find logs/local_setup.json${NC}"
    exit 1
fi

# Get the fork block number from anvil logs if in docker mode
if [ "$ENV_TYPE" = "docker" ]; then
    # Wait briefly for anvil to start and output its logs
    sleep 2
    
    # Get block number from docker logs
    BLOCK_NUMBER=$(docker logs anvil-1 2>&1 | grep "Block number:" | awk '{print $3}')
    
    if [ -n "$BLOCK_NUMBER" ]; then
        echo -e "${YELLOW}Found fork block number: $BLOCK_NUMBER${NC}"
        update_json_config "${ROOT_DIR}/${CONFIG_DIR}/anvil.messaging.docker.json" "$SN_MESSAGING" "$BLOCK_NUMBER"
    else
        echo -e "${RED}Could not find fork block number in anvil logs${NC}"
        update_json_config "${ROOT_DIR}/${CONFIG_DIR}/anvil.messaging.docker.json" "$SN_MESSAGING" "0"
    fi
else
    update_json_config "${ROOT_DIR}/${CONFIG_DIR}/anvil.messaging.json" "$SN_MESSAGING" "0"
fi

# Source the updated environment variables - use full path
source "${ROOT_DIR}/${ENV_FILES[0]}"

echo -e "${BLUE}Using L1_MESSAGE_SENDER: $L1_MESSAGE_SENDER${NC}"
echo -e "${BLUE}Using SN_MESSAGING: $SN_MESSAGING${NC}"
echo -e "${GREEN}${BOLD}Ethereum deployment completed successfully!${NC}" 