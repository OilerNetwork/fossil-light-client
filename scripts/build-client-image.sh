#!/bin/bash
set -e

# Navigate to the project root directory
cd "$(git rev-parse --show-toplevel)"

# Build the binary locally
echo "Building client binary locally..."
cargo build --release --package client --bin client

# Build the methods
echo "Building RISC0 methods locally..."
cd crates/methods && cargo build --release && cd ../..

# Find the methods directory
METHODS_DIR=$(find target/release/build -name "methods-*" -type d | head -n 1)
if [ -z "$METHODS_DIR" ]; then
    echo "Error: Could not find methods directory in target/release/build"
    exit 1
fi

echo "Found methods directory: $METHODS_DIR"

# Build the Docker image (without copying files yet)
echo "Building base Docker image..."
docker build -t fossil-client:base -f docker/Dockerfile.client .

# Create a temporary container
echo "Creating temporary container..."
CONTAINER_ID=$(docker create fossil-client:base)

# Make the binary executable locally first
echo "Making binary executable locally..."
chmod +x target/release/client

# Copy the binary and method ELFs to the container
echo "Copying binary to container..."
docker cp target/release/client $CONTAINER_ID:/usr/local/bin/client

echo "Copying method ELFs to container..."
if [ -d "$METHODS_DIR/out" ]; then
    docker cp $METHODS_DIR/out/. $CONTAINER_ID:/app/target/release/build/methods/out/
else
    echo "Warning: Method ELFs directory not found at $METHODS_DIR/out"
    # Create an empty directory to avoid errors
    mkdir -p tmp_methods_out
    docker cp tmp_methods_out/. $CONTAINER_ID:/app/target/release/build/methods/out/
    rm -rf tmp_methods_out
fi

# Commit the container as the final image directly
echo "Committing container as final image..."
docker commit $CONTAINER_ID fossil-client:with-files

# Remove the temporary container
echo "Cleaning up temporary container..."
docker rm $CONTAINER_ID

# Keep the CMD from Dockerfile for ECS compatibility
echo "Final image ready with CMD wrapper..."

# Clean up intermediate images
echo "Cleaning up intermediate images..."
# Don't remove the base image as it might be in use
# docker rmi fossil-client:base || true

echo "Done! The fossil-client:with-files image is now ready."