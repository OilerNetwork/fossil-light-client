#!/bin/bash

# Ensure the script stops on the first error
set -e

# Load environment variables
source ./.env

# Declare the first contract
echo "Declaring Universal ECIP contract..."
ECIP_HASH=$(starkli declare ../target/dev/risc0_bn254_verifier_UniversalECIP.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $ECIP_HASH"

echo "Declaring Groth16 Verifier contract..."
VERIFIER_HASH=$(starkli declare ../target/dev/risc0_bn254_verifier_Risc0Groth16VerifierBN254.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $VERIFIER_HASH"

# Deploy the first contract with salt
echo "Deploying Groth16 Verifier contract..."
VERIFIER_ADDRESS=$(starkli deploy $VERIFIER_HASH $ECIP_HASH --salt 1 | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract deployed at: $VERIFIER_ADDRESS"

# Path to the .env file
ENV_FILE="../../.env"

# Check if VERIFIER_ADDRESS already exists, and replace or append accordingly
if grep -q "^VERIFIER_ADDRESS=" "$ENV_FILE"; then
    echo "VERIFIER_ADDRESS already exists, replacing..."
    sed -i "s/^VERIFIER_ADDRESS=.*/VERIFIER_ADDRESS=$VERIFIER_ADDRESS/" "$ENV_FILE"
else
    echo "Appending VERIFIER_ADDRESS to $ENV_FILE..."
    echo "VERIFIER_ADDRESS=$VERIFIER_ADDRESS" >> "$ENV_FILE"
fi

echo "Verifier address successfully updated in $ENV_FILE"
