#!/bin/bash

# Ensure the script stops on the first error
set -e

# Store the original directory (now inside container at /app)
ORIGINAL_DIR="/app"

# Check if environment argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <environment>"
    echo "Available environments: local, sepolia, mainnet"
    exit 1
fi

# Validate environment argument
ENV_TYPE="$1"
case "$ENV_TYPE" in
    "local"|"sepolia"|"mainnet")
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

ETHEREUM_DIR="/app/contracts/ethereum"
STARKNET_DIR="/app/contracts/starknet"

# Deploy Ethereum contracts
cd "$ETHEREUM_DIR"
forge script script/LocalTesting.s.sol:LocalSetup --broadcast --rpc-url $ANVIL_URL

L1_MESSAGE_SENDER=0x364C7188028348566E38D762f6095741c49f492B

# Now deploy Starknet contracts
echo "Deploying Starknet contracts..."
cd "$STARKNET_DIR"

scarb build

# Declare and deploy Fossil Store contract
echo "Declaring Fossil Store contract..."
FOSSILSTORE_HASH=$(starkli declare ./target/dev/fossil_store_Store.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $FOSSILSTORE_HASH"

echo "Deploying Fossil Store contract..."
FOSSILSTORE_ADDRESS=$(starkli deploy $FOSSILSTORE_HASH --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract address: $FOSSILSTORE_ADDRESS"

# Declare and deploy Fossil L1MessageProxy contract
echo "Declaring Fossil L1MessageProxy contract..."
L1MESSAGEPROXY_HASH=$(starkli declare ./target/dev/l1_message_proxy_L1MessageProxy.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $L1MESSAGEPROXY_HASH"

echo "Deploying Fossil L1MessageProxy contract..."
L1MESSAGEPROXY_ADDRESS=$(starkli deploy $L1MESSAGEPROXY_HASH $L1_MESSAGE_SENDER $FOSSILSTORE_ADDRESS --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
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
VERIFIER_ADDRESS=$(starkli deploy $VERIFIER_HASH $ECIP_HASH --salt 1 | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract deployed at: $VERIFIER_ADDRESS"

echo "Declaring Fossil Verifier contract..."
FOSSIL_VERIFIER_HASH=$(starkli declare ./target/dev/verifier_FossilVerifier.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $FOSSIL_VERIFIER_HASH"

echo "Deploying Fossil Verifier contract..."
FOSSIL_VERIFIER_ADDRESS=$(starkli deploy $FOSSIL_VERIFIER_HASH $VERIFIER_ADDRESS $FOSSILSTORE_ADDRESS --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract deployed at: $FOSSIL_VERIFIER_ADDRESS"

echo "All contracts deployed!"

# Update the environment file with new addresses
update_env_var() {
    local var_name=$1
    local var_value=$2
    
    if grep -q "^$var_name=" "$ENV_FILE"; then
        echo "$var_name already exists, replacing in $ENV_FILE..."
        sed -i "s|^$var_name=.*|$var_name=$var_value|" "$ENV_FILE"
    else
        echo "Appending $var_name to $ENV_FILE..."
        echo "$var_name=$var_value" >> "$ENV_FILE"
    fi
}

# Update the environment file with the new addresses
update_env_var "L2_MSG_PROXY" "$L1MESSAGEPROXY_ADDRESS"
update_env_var "FOSSIL_STORE" "$FOSSILSTORE_ADDRESS"
update_env_var "STARKNET_VERIFIER" "$VERIFIER_ADDRESS"
update_env_var "FOSSIL_VERIFIER" "$FOSSIL_VERIFIER_ADDRESS"

# Return to original directory
cd "$ORIGINAL_DIR"

# Source the updated environment file
source "$ENV_FILE"

echo "Environment variables successfully updated in $ENV_FILE"

