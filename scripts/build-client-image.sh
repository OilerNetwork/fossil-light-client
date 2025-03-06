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

# Commit the container with the copied files
echo "Committing container with copied files..."
docker commit $CONTAINER_ID fossil-client:with-files

# Remove the temporary container
echo "Cleaning up first temporary container..."
docker rm $CONTAINER_ID

# Create and start a new container to make the binary executable
echo "Creating and starting a new container to make binary executable..."
CONTAINER_ID=$(docker run -d fossil-client:with-files sleep 30)

# Make the binary executable
echo "Making binary executable..."
docker exec $CONTAINER_ID chmod +x /usr/local/bin/client

# Verify the binary exists
echo "Verifying binary exists in container..."
docker exec $CONTAINER_ID ls -la /usr/local/bin/client || echo "Warning: Binary not found in container"

# Check GLIBC version in container
echo "Checking GLIBC version in container..."
docker exec $CONTAINER_ID ldd --version || echo "Warning: Could not check GLIBC version"

# Commit the container as the final image
echo "Committing changes to final image..."
docker commit $CONTAINER_ID fossil-client:latest

# Stop and remove the temporary container
echo "Cleaning up second temporary container..."
docker stop $CONTAINER_ID
docker rm $CONTAINER_ID

# Update the entrypoint in the final image
echo "Updating entrypoint in final image..."
docker build -t fossil-client:latest - <<EOF
FROM fossil-client:latest
ENTRYPOINT ["/usr/local/bin/client"]
EOF

# Clean up intermediate images
echo "Cleaning up intermediate images..."
docker rmi fossil-client:with-files || true
# Don't remove the base image as it might be in use
# docker rmi fossil-client:base || true

echo "Done! The fossil-client:latest image is now ready."