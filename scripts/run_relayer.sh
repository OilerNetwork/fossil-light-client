#!/bin/sh

set -e

# Check environment and set interval
if [ "$ENV_FILE" = ".env.local" ]; then
    INTERVAL_MINUTES=10
    MAX_RETRIES=3
    RETRY_DELAY=10  # seconds
else
    INTERVAL_MINUTES=720  # 12 hours = 720 minutes
    MAX_RETRIES=5
    RETRY_DELAY=60  # seconds
fi

run_with_retry() {
    attempt=1
    while [ $attempt -le $MAX_RETRIES ]; do
        echo "Starting relayer (attempt $attempt/$MAX_RETRIES)..."
        if /usr/local/bin/relayer; then
            return 0
        fi
        
        exit_code=$?
        echo "Relayer exited with code $exit_code"
        
        if [ $attempt -lt $MAX_RETRIES ]; then
            echo "Retrying in $RETRY_DELAY seconds..."
            sleep $RETRY_DELAY
        fi
        attempt=$((attempt + 1))
    done
    
    echo "Failed to run relayer after $MAX_RETRIES attempts"
    return 1
}

while true; do
    if run_with_retry; then
        echo "Relayer completed successfully. Waiting $INTERVAL_MINUTES minutes before next run..."
        i=$INTERVAL_MINUTES
        while [ $i -gt 0 ]; do
            echo "Next run in $i minutes..."
            sleep 60
            i=$((i - 1))
        done
    else
        echo "Relayer failed all retry attempts. Exiting..."
        exit 1
    fi
done
