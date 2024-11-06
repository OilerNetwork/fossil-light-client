#!/bin/bash

# Ensure the script stops on the first error
set -e

L1_MESSAGES_SENDER="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"


# Load environment variables
source ./katana/katana.env

# Declare the first contract
echo "Declaring Fossil Store  contract..."
FOSSILSTORE_HASH=$(starkli declare ./target/dev/store_Store.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $FOSSILSTORE_HASH"

echo "Deploying Fossil Store contract..."
FOSSILSTORE_ADDRESS=$(starkli deploy $FOSSILSTORE_HASH --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract address: $FOSSILSTORE_ADDRESS"

echo "Declaring Fossil L1MessageProxy contract..."  
L1MESSAGEPROXY_HASH=$(starkli declare ./target/dev/l1_message_proxy_L1MessageProxy.contract_class.json --compiler-version 2.8.2 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Class hash declared: $L1MESSAGEPROXY_HASH"

echo "Deploying Fossil L1MessageProxy contract..."
L1MESSAGEPROXY_ADDRESS=$(starkli deploy $L1MESSAGEPROXY_HASH $L1_MESSAGES_SENDER $FOSSILSTORE_ADDRESS --salt 1 -w | grep -o '0x[a-fA-F0-9]\{64\}' | head -1)
echo "Contract address: $L1MESSAGEPROXY_ADDRESS"

echo "Done!"