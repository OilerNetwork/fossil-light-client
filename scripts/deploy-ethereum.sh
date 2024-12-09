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
"local" | "sepolia" | "mainnet")
    ENV_FILE="/app/.env.$ENV_TYPE"
    echo "Using environment: $ENV_TYPE ($ENV_FILE)"
    ;;
*)
    echo "Invalid environment. Must be one of: local, sepolia, mainnet"
    exit 1
    ;;
esac

# Source the appropriate environment file
source "$ENV_FILE"
export ACCOUNT_PRIVATE_KEY=${ACCOUNT_PRIVATE_KEY}

ETHEREUM_DIR="/app/contracts/ethereum"
CONFIG_DIR="/app/config"

# Define colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'
BOLD='\033[1m'
RED='\033[0;31m'

# Function to update environment variables
update_env_var() {
    local var_name=$1
    local var_value=$2

    if grep -q "^$var_name=" "$ENV_FILE"; then
        echo -e "${BLUE}$var_name already exists, replacing in $ENV_FILE...${NC}"
        sed -i "s|^$var_name=.*|$var_name=$var_value|" "$ENV_FILE"
    else
        echo -e "${BLUE}Appending $var_name to $ENV_FILE...${NC}"
        echo "$var_name=$var_value" >>"$ENV_FILE"
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
        
        if forge script script/LocalTesting.s.sol:LocalSetup --broadcast --rpc-url $ANVIL_URL; then
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

# Deploy Ethereum contracts
cd "$ETHEREUM_DIR"
deploy_contracts || exit 1

# Read values from the JSON file and update env vars
SN_MESSAGING=$(jq -r '.snMessaging_address' logs/local_setup.json)
L1_MESSAGE_SENDER=$(jq -r '.l1MessageSender_address' logs/local_setup.json)

# Update the environment variables
update_env_var "SN_MESSAGING" "$SN_MESSAGING"
update_env_var "L1_MESSAGE_SENDER" "$L1_MESSAGE_SENDER"

# Update the anvil.messaging.json config
update_json_config "$CONFIG_DIR/anvil.messaging.json" "$SN_MESSAGING"

# Source the updated environment variables
source "$ENV_FILE"

echo -e "${BLUE}Using L1_MESSAGE_SENDER: $L1_MESSAGE_SENDER${NC}"
echo -e "${BLUE}Using SN_MESSAGING: $SN_MESSAGING${NC}"
echo -e "${GREEN}${BOLD}Ethereum deployment completed successfully!${NC}" 