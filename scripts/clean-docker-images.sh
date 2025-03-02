#!/bin/bash
set -e

# Default options
FORCE_REMOVE=false
REMOVE_CONTAINERS=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --force|-f)
            FORCE_REMOVE=true
            shift
            ;;
        --remove-containers|-c)
            REMOVE_CONTAINERS=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo "Clean up Docker images, including dangling and untagged (<none>) images."
            echo ""
            echo "Options:"
            echo "  --force, -f              Force remove images (use with caution)"
            echo "  --remove-containers, -c  Remove stopped containers before cleaning images"
            echo "  --help, -h               Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Optionally remove stopped containers first
if [ "$REMOVE_CONTAINERS" = true ]; then
    echo "Removing all stopped containers..."
    STOPPED_CONTAINERS=$(docker ps -a -q -f status=exited)
    if [ -n "$STOPPED_CONTAINERS" ]; then
        docker rm $STOPPED_CONTAINERS
        echo "Successfully removed stopped containers."
    else
        echo "No stopped containers found."
    fi
fi

echo "Removing all dangling Docker images..."
docker image prune -f

echo "Removing all untagged (<none>) images..."
# Find all images with <none> tag
NONE_IMAGES=$(docker images -f "dangling=true" -q)
if [ -n "$NONE_IMAGES" ]; then
    # Set force flag if requested
    FORCE_FLAG=""
    if [ "$FORCE_REMOVE" = true ]; then
        echo "WARNING: Force removing images. This may cause issues if images are used by containers."
        FORCE_FLAG="--force"
    fi
    
    # Try to remove images, but don't exit on error
    set +e
    docker rmi $FORCE_FLAG $NONE_IMAGES
    RMI_EXIT_CODE=$?
    set -e
    
    if [ $RMI_EXIT_CODE -eq 0 ]; then
        echo "Successfully removed untagged images."
    else
        echo ""
        echo "Some images could not be removed because they are in use by containers."
        echo "You can:"
        echo "  1. Use --remove-containers (-c) to remove stopped containers first"
        echo "  2. Use --force (-f) to force remove images (use with caution)"
        echo "  3. Stop and remove containers manually before running this script"
    fi
else
    echo "No untagged images found."
fi

echo "Listing remaining Docker images..."
docker images

echo "Done! Cleanup of Docker images completed." 