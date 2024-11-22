#!/bin/bash

set -e

cd ../crates/relayer

while true; do
    cargo run
    echo "Waiting 60 seconds before next run..."
    sleep 60
done
