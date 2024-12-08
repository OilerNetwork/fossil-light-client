#!/bin/sh

set -e

# Check environment and set interval
if [ "$ENV_FILE" = ".env.local" ]; then
    INTERVAL_MINUTES=10
else
    INTERVAL_MINUTES=720  # 12 hours = 720 minutes
fi

while true; do
    /usr/local/bin/relayer
    echo "Waiting $INTERVAL_MINUTES minutes before next run..."
    i=$INTERVAL_MINUTES
    while [ $i -gt 0 ]; do
        echo "Next run in $i minutes..."
        sleep 60
        i=$((i - 1))
    done
done
