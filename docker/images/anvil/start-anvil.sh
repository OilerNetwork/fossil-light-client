#!/bin/bash

echo "Environment variables:"
echo "ETH_RPC_URL: ${ETH_RPC_URL}"
echo "FOUNDRY_EVM_VERSION: ${FOUNDRY_EVM_VERSION}"

if [ -z "${ETH_RPC_URL}" ]; then
    echo "Error: ETH_RPC_URL is not set"
    exit 1
fi

exec anvil \
    --fork-url "${ETH_RPC_URL}" \
    --block-time 12 \
    --host 0.0.0.0 \
    --port 8545 \