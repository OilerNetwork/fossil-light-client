# Use the official Rust image as a base
FROM rust:latest

# Set the working directory
WORKDIR /app

# Copy the entire relayer project into the container
COPY . .

# Build the Rust project
RUN cargo build --release

# Set the command to run your application
CMD ["cargo", "run", "--release"]
