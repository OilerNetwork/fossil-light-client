FROM ghcr.io/foundry-rs/foundry

WORKDIR /app

# Install curl for health checks
RUN apk add --no-cache curl bash

# Create deployment script
COPY <<'EOF' /app/deploy.sh
#!/bin/bash
set -e

echo "=== Deployment Environment ==="
echo "ETH_RPC_URL: ${ETH_RPC_URL}"
echo "FOUNDRY_EVM_VERSION: ${FOUNDRY_EVM_VERSION}"
echo "PRIVATE_KEY: ${PRIVATE_KEY}"

# Test RPC connection directly
echo "Testing RPC connection..."
curl -X POST \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
    "${ETH_RPC_URL}"

echo "=== Checking File Structure ==="
ls -la /app/contracts/ethereum/script/
echo "=== File Contents ==="
cat /app/contracts/ethereum/script/LocalTesting.s.sol

echo "=== Running Deployment ==="
cd /app/contracts/ethereum && forge script script/LocalTesting.s.sol:LocalSetup \
    --rpc-url "${ETH_RPC_URL}" \
    --private-key "${PRIVATE_KEY}" \
    --broadcast
EOF

RUN chmod +x /app/deploy.sh

CMD ["/app/deploy.sh"]