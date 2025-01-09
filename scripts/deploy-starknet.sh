#!/bin/bash

# Ensure the script stops on the first error
set -e

# Store the original directory (works both in container and local environment)
ORIGINAL_DIR="$(pwd)"
UPDATE_INTERVAL=0

# Update the environment file with new addresses
update_env_var() {
    local env_file=$1
    local var_name=$2
    local var_value=$3
    
    if grep -q "^$var_name=" "$env_file"; then
        echo -e "${BLUE}$var_name already exists, replacing in $env_file...${NC}"
        sed -i "s|^$var_name=.*|$var_name=$var_value|" "$env_file"
    else
        echo -e "${BLUE}Appending $var_name to $env_file...${NC}"
        echo "$var_name=$var_value" >>"$env_file"
    fi
}

# Check if environment argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <environment>"
    echo "Available environments: local, sepolia, mainnet, docker"
    exit 1
fi

# Validate environment argument
ENV_TYPE="$1"
case "$ENV_TYPE" in
    "local" | "sepolia" | "mainnet")
        ENV_FILES=("$ORIGINAL_DIR/.env.$ENV_TYPE")
        echo "Using environment: $ENV_TYPE (${ENV_FILES[0]})"
    ;;
    "docker")
        # Update docker env first, then copy values to local env
        ENV_FILES=("$ORIGINAL_DIR/.env.docker")
        SECONDARY_ENV="$ORIGINAL_DIR/.env.local"
        echo "Using environment: $ENV_TYPE (updating ${ENV_FILES[0]} and will sync to $SECONDARY_ENV)"
    ;;
    *)
        echo "Invalid environment. Must be one of: local, sepolia, mainnet, docker"
        exit 1
    ;;
esac

# Check if environment files exist
for env_file in "${ENV_FILES[@]}"; do
    if [ ! -f "$env_file" ]; then
        echo "Error: Environment file $env_file not found"
        exit 1
    fi
done

# Source the primary environment file
source "${ENV_FILES[0]}"

STARKNET_DIR="$ORIGINAL_DIR/contracts/starknet"

# Define colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color
BOLD='\033[1m'
RED='\033[0;31m'

# Now deploy Starknet contracts
echo -e "\n${BLUE}${BOLD}Building Starknet contracts...${NC}"
cd "$STARKNET_DIR"

scarb build

echo -e "\n${BLUE}${BOLD}Deploying Starknet contracts...${NC}"
# Declare and deploy Fossil Store contract
echo -e "\n${YELLOW}Declaring Fossil Store contract...${NC}"
FOSSILSTORE_HASH=$(starkli declare ./target/dev/fossil_store_Store.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --compiler-version 2.9.1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$FOSSILSTORE_HASH${NC}"
echo

echo -e "${YELLOW}Deploying Fossil Store contract...${NC}"
FOSSILSTORE_ADDRESS=$(starkli deploy $FOSSILSTORE_HASH --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract address: ${BOLD}$FOSSILSTORE_ADDRESS${NC}"
echo

# Declare and deploy Fossil L1MessageProxy contract
echo -e "${YELLOW}Declaring Fossil L1MessageProxy contract...${NC}"
L1MESSAGEPROXY_HASH=$(starkli declare ./target/dev/l1_message_proxy_L1MessageProxy.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --compiler-version 2.9.1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$L1MESSAGEPROXY_HASH${NC}"
echo

echo -e "${YELLOW}Deploying Fossil L1MessageProxy contract...${NC}"
L1MESSAGEPROXY_ADDRESS=$(starkli deploy $L1MESSAGEPROXY_HASH $L1_MESSAGE_SENDER $FOSSILSTORE_ADDRESS --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract address: ${BOLD}$L1MESSAGEPROXY_ADDRESS${NC}"
echo

# Declare and deploy Universal ECIP contract
echo -e "${YELLOW}Declaring Universal ECIP contract...${NC}"
ECIP_HASH=$(starkli declare ./target/dev/verifier_UniversalECIP.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --compiler-version 2.9.1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$ECIP_HASH${NC}"
echo

# Declare and deploy Groth16 Verifier contract
echo -e "${YELLOW}Declaring Groth16 Verifier contract...${NC}"
VERIFIER_HASH=$(starkli declare ./target/dev/verifier_Risc0Groth16VerifierBN254.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --compiler-version 2.9.1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$VERIFIER_HASH${NC}"
echo

echo -e "${YELLOW}Deploying Groth16 Verifier contract...${NC}"
VERIFIER_ADDRESS=$(starkli deploy $VERIFIER_HASH $ECIP_HASH --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --salt 1 | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract deployed at: ${BOLD}$VERIFIER_ADDRESS${NC}"
echo

echo -e "${YELLOW}Declaring Fossil Verifier contract...${NC}"
FOSSIL_VERIFIER_HASH=$(starkli declare ./target/dev/verifier_FossilVerifier.contract_class.json --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --compiler-version 2.9.1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Class hash declared: ${BOLD}$FOSSIL_VERIFIER_HASH${NC}"
echo

echo -e "${YELLOW}Deploying Fossil Verifier contract...${NC}"
FOSSIL_VERIFIER_ADDRESS=$(starkli deploy $FOSSIL_VERIFIER_HASH $VERIFIER_ADDRESS $FOSSILSTORE_ADDRESS --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo -e "${GREEN}Contract deployed at: ${BOLD}$FOSSIL_VERIFIER_ADDRESS${NC}"
echo

echo -e "${YELLOW}Initializing Fossil Store contract...${NC}"
starkli invoke $FOSSILSTORE_ADDRESS initialize $FOSSIL_VERIFIER_ADDRESS $UPDATE_INTERVAL --account $STARKNET_ACCOUNT --rpc $STARKNET_RPC_URL -w
echo -e "${GREEN}Fossil Store contract initialized${NC}"
echo

echo -e "\n${GREEN}${BOLD}All contracts deployed!${NC}"

# Update the environment files with the new addresses
for env_file in "${ENV_FILES[@]}"; do
    if [ ! -f "$env_file" ]; then
        echo -e "${RED}Warning: $env_file not found, skipping...${NC}"
        continue
    fi
    update_env_var "$env_file" "L2_MSG_PROXY" "$L1MESSAGEPROXY_ADDRESS"
    update_env_var "$env_file" "FOSSIL_STORE" "$FOSSILSTORE_ADDRESS"
    update_env_var "$env_file" "STARKNET_VERIFIER" "$VERIFIER_ADDRESS"
    update_env_var "$env_file" "FOSSIL_VERIFIER" "$FOSSIL_VERIFIER_ADDRESS"
done

# If in docker mode, sync the addresses to .env.local
if [ "$ENV_TYPE" = "docker" ] && [ -f "$SECONDARY_ENV" ]; then
    echo -e "${BLUE}Syncing addresses to $SECONDARY_ENV...${NC}"
    update_env_var "$SECONDARY_ENV" "L2_MSG_PROXY" "$L1MESSAGEPROXY_ADDRESS"
    update_env_var "$SECONDARY_ENV" "FOSSIL_STORE" "$FOSSILSTORE_ADDRESS"
    update_env_var "$SECONDARY_ENV" "STARKNET_VERIFIER" "$VERIFIER_ADDRESS"
    update_env_var "$SECONDARY_ENV" "FOSSIL_VERIFIER" "$FOSSIL_VERIFIER_ADDRESS"
fi

# Return to original directory
cd "$ORIGINAL_DIR"

# Source the updated primary environment file
source "${ENV_FILES[0]}"

sleep 5

echo -e "${GREEN}${BOLD}Environment variables successfully updated in ${ENV_FILES[0]}${NC}"

# Reset ownership of generated files back to the host user
if [ -n "$HOST_UID" ] && [ -n "$HOST_GID" ]; then
    chown -R $HOST_UID:$HOST_GID \
        "$ORIGINAL_DIR/contracts/starknet/target" \
        "$ORIGINAL_DIR/logs" \
        "$ORIGINAL_DIR/config" \
        "$ORIGINAL_DIR/.env.local" \
        "$ORIGINAL_DIR/.env.docker" \
        "$ORIGINAL_DIR/.env"
fi