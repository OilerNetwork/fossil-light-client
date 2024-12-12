#!/bin/bash

# Check if environment file argument is provided
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <env-file>"
    echo "Example: $0 .env.sepolia"
    exit 1
fi

# Ensure the script stops on the first error
set -e

# Store the root directory path and env file path
ROOT_DIR=$(pwd)
ENV_FILE="$1"
UPDATE_INTERVAL=40

# Check if env file exists
if [ ! -f "$ENV_FILE" ]; then
    echo "Error: Environment file $ENV_FILE not found"
    exit 1
fi

source "$ENV_FILE"
cat "$ENV_FILE"

# Define color codes
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# Now deploy Starknet contracts
echo "Deploying Starknet contracts..."

# Export Starknet environment variables
export STARKNET_ACCOUNT=${STARKNET_ACCOUNT}
export STARKNET_KEYSTORE=${STARKNET_KEYSTORE}
export STARKNET_RPC=${STARKNET_RPC_URL}

# Log the exported variables (masking private key for security)
echo "STARKNET_ACCOUNT: ${STARKNET_ACCOUNT}"
echo "STARKNET_KEYSTORE: ${STARKNET_KEYSTORE}"
echo "STARKNET_RPC: ${STARKNET_RPC}"

cd contracts/starknet
scarb build

# Declare and deploy Fossil Store contract
echo "Declaring Fossil Store contract..."
FOSSILSTORE_HASH=$(starkli declare ./target/dev/fossil_store_Store.contract_class.json \
    --compiler-version 2.8.2 \
    --account $STARKNET_ACCOUNT \
    -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $FOSSILSTORE_HASH"

echo "Deploying Fossil Store contract..."
FOSSILSTORE_ADDRESS=$(starkli deploy $FOSSILSTORE_HASH \
    --account $STARKNET_ACCOUNT \
    -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract address: $FOSSILSTORE_ADDRESS"

# Declare and deploy Fossil L1MessageProxy contract
echo "Declaring Fossil L1MessageProxy contract..."
L1MESSAGEPROXY_HASH=$(starkli declare ./target/dev/l1_message_proxy_L1MessageProxy.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $L1MESSAGEPROXY_HASH"

echo "Deploying Fossil L1MessageProxy contract..."
L1MESSAGEPROXY_ADDRESS=$(starkli deploy $L1MESSAGEPROXY_HASH $L1_MESSAGE_SENDER $FOSSILSTORE_ADDRESS  -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract address: $L1MESSAGEPROXY_ADDRESS"

# Declare and deploy Universal ECIP contract
echo "Declaring Universal ECIP contract..."
ECIP_HASH=$(starkli declare ./target/dev/verifier_UniversalECIP.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $ECIP_HASH"

# Declare and deploy Groth16 Verifier contract
echo "Declaring Groth16 Verifier contract..."
VERIFIER_HASH=$(starkli declare ./target/dev/verifier_Risc0Groth16VerifierBN254.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $VERIFIER_HASH"

echo "Deploying Groth16 Verifier contract..."
VERIFIER_ADDRESS=$(starkli deploy $VERIFIER_HASH $ECIP_HASH -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract deployed at: $VERIFIER_ADDRESS"

echo "Declaring Fossil Verifier contract..."
FOSSIL_VERIFIER_HASH=$(starkli declare ./target/dev/verifier_FossilVerifier.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $FOSSIL_VERIFIER_HASH"

echo "Deploying Fossil Verifier contract..."
FOSSIL_VERIFIER_ADDRESS=$(starkli deploy $FOSSIL_VERIFIER_HASH $VERIFIER_ADDRESS $FOSSILSTORE_ADDRESS  -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract deployed at: $FOSSIL_VERIFIER_ADDRESS"

if [ "$DEPLOYMENT_VERSION" = "local" ] || [ "$DEPLOYMENT_VERSION" = "sepolia" ]; then
    echo "Initializing Fossil Store contract for $DEPLOYMENT_VERSION deployment..."
    starkli invoke $FOSSILSTORE_ADDRESS initialize $STARKNET_ACCOUNT_ADDRESS $UPDATE_INTERVAL -w
else
    echo "Error: Unsupported DEPLOYMENT_VERSION: $DEPLOYMENT_VERSION"
    echo "Supported versions are: local, sepolia"
    exit 1
fi
echo "Fossil Store contract initialized"
echo

echo "All contracts deployed!"

# Return to root directory
cd "$ROOT_DIR"

# Update the environment variables
update_env_var "L2_MSG_PROXY" "$L1MESSAGEPROXY_ADDRESS"
update_env_var "FOSSIL_STORE" "$FOSSILSTORE_ADDRESS"
update_env_var "STARKNET_VERIFIER" "$VERIFIER_ADDRESS"
update_env_var "FOSSIL_VERIFIER" "$FOSSIL_VERIFIER_ADDRESS"

source "$ENV_FILE"

echo "Environment variables successfully updated in $ENV_FILE"