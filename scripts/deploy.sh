#!/bin/bash

# Ensure the script stops on the first error
set -e

WORKING_DIR="../contracts/starknet/"
SOLIDITY_DIR="../contracts/ethereum/"

# Load environment variables
source ../.env
source ../config/katana.env

# First deploy Solidity contracts
echo "Deploying Solidity contracts..."
cd $SOLIDITY_DIR

# Build and deploy Solidity contracts
forge build
forge create --unlocked --from $DEPLOYER_ADDRESS src/FossilL2MessageReceiver.sol:FossilL2MessageReceiver
L1_MESSAGES_SENDER=$(cast send --unlocked --from $DEPLOYER_ADDRESS --create $(cat out/FossilL2MessageReceiver.sol/FossilL2MessageReceiver.json | jq -r .bytecode.object) | grep -o '0x[a-fA-F0-9]\{40\}')
echo "L1 Message Receiver deployed at: $L1_MESSAGES_SENDER"

# Now deploy Starknet contracts
echo "Deploying Starknet contracts..."
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

# Fetch the current Ethereum block number using `cast`
ETH_BLOCK=$(cast block-number)
echo "Current Ethereum block number: $ETH_BLOCK"

# Ensure `ETH_BLOCK` is a valid number before performing arithmetic
if [[ $ETH_BLOCK =~ ^[0-9]+$ ]]; then
    # Subtract 256 from the current block number
    ETH_BLOCK=$((ETH_BLOCK - 256))
    echo "Updated Ethereum block number: $ETH_BLOCK"
    
    # Run the Starkli command with the updated block number
    starkli invoke $FOSSILSTORE_ADDRESS update_mmr_state $ETH_BLOCK 0x0
    echo "Updated MMR state on Starknet for testing with block number: $ETH_BLOCK"
else
    echo "Failed to retrieve a valid block number from 'cast'."
fi

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
        echo "$var_name=$var_value" >>"$ENV_FILE"
    fi
}

# Update the .env file with the new addresses
update_env_var "L2_MSG_PROXY" "$L1MESSAGEPROXY_ADDRESS"
update_env_var "FOSSIL_STORE" "$FOSSILSTORE_ADDRESS"
update_env_var "STARKNET_VERIFIER" "$VERIFIER_ADDRESS"

source ../../.env

echo "Environment variables successfully updated in $ENV_FILE"