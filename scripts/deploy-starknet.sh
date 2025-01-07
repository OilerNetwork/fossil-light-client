#!/bin/bash

# Ensure the script stops on the first error
set -e

# Store the original directory (works both in container and local environment)
ORIGINAL_DIR="$(pwd)"
UPDATE_INTERVAL=0

# Update the environment file with new addresses
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

# Check if environment argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <environment>"
    echo "Available environments: local, sepolia, mainnet, docker"
    exit 1
fi

# Validate environment argument
ENV_TYPE="$1"
case "$ENV_TYPE" in
    "local" | "sepolia" | "mainnet" | "docker")
        ENV_FILE="$ORIGINAL_DIR/.env.$ENV_TYPE"
        echo "Using environment: $ENV_TYPE ($ENV_FILE)"
    ;;
    *)
        echo "Invalid environment. Must be one of: local, sepolia, mainnet"
        exit 1
    ;;
esac

# Check if environment file exists
if [ ! -f "$ENV_FILE" ]; then
    echo "Error: Environment file $ENV_FILE not found"
    exit 1
fi

# Source the appropriate environment file
source "$ENV_FILE"

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

# Update the environment file with the new addresses
update_env_var "L2_MSG_PROXY" "$L1MESSAGEPROXY_ADDRESS"
update_env_var "FOSSIL_STORE" "$FOSSILSTORE_ADDRESS"
update_env_var "STARKNET_VERIFIER" "$VERIFIER_ADDRESS"
update_env_var "FOSSIL_VERIFIER" "$FOSSIL_VERIFIER_ADDRESS"

# Return to original directory
cd "$ORIGINAL_DIR"

# Source the updated environment file
source "$ENV_FILE"

sleep 5

echo -e "${GREEN}${BOLD}Environment variables successfully updated in $ENV_FILE${NC}"