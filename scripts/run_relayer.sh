#!/bin/bash

set -e

cd ../crates/relayer

while true; do
    cargo run
    echo "Waiting 180 seconds before next run..."
    sleep 180
done
