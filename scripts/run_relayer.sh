#!/bin/bash

set -e

cd ../crates/relayer

while true; do
    cargo run --release
    echo "Waiting 180 seconds before next run..."
    sleep 180
done
