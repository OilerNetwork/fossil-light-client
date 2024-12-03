#!/bin/bash

set -e

cd crates/relayer

while true; do
    cargo run --release
    echo "Waiting 10 minutes before next run..."
    for ((i=10; i>0; i--)); do
        echo "Next run in $i minutes..."
        sleep 60
    done
done
