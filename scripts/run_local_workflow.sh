# Function to wait for a specific log message in a given log file
wait_for_log() {
  local log_file=$1
  local search_string=$2

  echo "Waiting for '$search_string' in $log_file..."
  while ! grep -q "$search_string" "$log_file"; do
    sleep 1
  done
  echo "Found '$search_string' in $log_file."
}

# Start Terminal 1: Start Anvil Ethereum Devnet
gnome-terminal -- bash -c "
cd config;
source anvil.env;
anvil --fork-url \$ETH_RPC_URL --auto-impersonate --block-time 12 | tee anvil.log;
exec bash"

# Wait for Anvil to be ready
wait_for_log "config/anvil.log" "Listening on"

# Start Terminal 2: Deploy L1MessageSender.sol
gnome-terminal -- bash -c "
cd contracts/ethereum;
cp ../../config/anvil.env .env;
source .env;
forge script script/LocalTesting.s.sol:LocalSetup --broadcast --rpc-url \$ANVIL_URL;
exec bash"

# Wait for the contract deployment to finish
sleep 10  # Adjust as needed

# Start Terminal 3: Start Katana Starknet Devnet
gnome-terminal -- bash -c "
cd scripts/katana;
source ../../config/katana.env;
katana --messaging ../../config/anvil.messaging.json --disable-fee | tee katana.log;
exec bash"

# Wait for Katana to be ready
wait_for_log "scripts/katana/katana.log" "RPC server started"

# Start Terminal 4: Deploy Starknet Contracts
gnome-terminal -- bash -c "
cd scripts/katana;
./deploy.sh | tee deploy.log;
exec bash"

# Wait for the "Environment variables successfully updated" message
wait_for_log "scripts/katana/deploy.log" "Environment variables successfully updated"

# Start Terminal 5: Run the Rust Relayer
gnome-terminal -- bash -c "
cp config/anvil.env .env
cd relayer;
cargo run;
exec bash"

# Wait for the Rust Relayer to start
sleep 5  # Adjust as needed

# Start Terminal 6: Run the Rust Light Client
gnome-terminal -- bash -c "
cd client;
cargo run;
exec bash"

echo "Local testing setup complete. All services are running."
