#!/bin/bash

# Ensure the script stops on the first error
set -e

WORKING_DIR="../../contracts/starknet/"
L1_MESSAGES_SENDER="0xb60971942E4528A811D24826768Bc91ad1383D21"

# Load environment variables
source ../../config/katana.env
cd $WORKING_DIR

scarb build

# Declare and deploy Fossil Store contract
echo "Declaring Fossil Store contract..."
FOSSILSTORE_HASH=$(starkli declare ./target/dev/store_Store.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $FOSSILSTORE_HASH"

echo "Deploying Fossil Store contract..."
FOSSILSTORE_ADDRESS=$(starkli deploy $FOSSILSTORE_HASH --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract address: $FOSSILSTORE_ADDRESS"

# Declare and deploy Fossil L1MessageProxy contract
echo "Declaring Fossil L1MessageProxy contract..."
L1MESSAGEPROXY_HASH=$(starkli declare ./target/dev/l1_message_proxy_L1MessageProxy.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $L1MESSAGEPROXY_HASH"

echo "Deploying Fossil L1MessageProxy contract..."
L1MESSAGEPROXY_ADDRESS=$(starkli deploy $L1MESSAGEPROXY_HASH $L1_MESSAGES_SENDER $FOSSILSTORE_ADDRESS --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
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

echo "All contracts deployed!"

# Path to the .env file
ENV_FILE="../../.env"

# Function to update or append an environment variable in the .env file
update_env_var() {
    local var_name=$1
    local var_value=$2
    if grep -q "^$var_name=" "$ENV_FILE"; then
        echo "$var_name already exists, replacing..."
        sed -i "s|^$var_name=.*|$var_name=$var_value|" "$ENV_FILE"
    else
        echo "Appending $var_name to $ENV_FILE..."
        echo "$var_name=$var_value" >> "$ENV_FILE"
    fi
}

# Update the .env file with the new addresses
update_env_var "L2_MSG_PROXY" "$L1MESSAGEPROXY_ADDRESS"
update_env_var "FOSSIL_STORE" "$FOSSILSTORE_ADDRESS"
update_env_var "STARKNET_VERIFIER" "$VERIFIER_ADDRESS"

echo "Environment variables successfully updated in $ENV_FILE"
