#!/bin/bash
set -e

echo "Removing all dangling Docker images..."
docker image prune -f

echo "Listing remaining Docker images..."
docker images

echo "Done! All dangling Docker images have been removed." 