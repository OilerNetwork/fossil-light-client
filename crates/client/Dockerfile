# Use the official Rust image as a base
FROM rust:latest

# Set the working directory
WORKDIR /app

# Copy the entire project into the container
COPY . .

# Copy the Cargo configuration
COPY .cargo/config.toml /usr/local/cargo/config.toml

# Configure Git to handle large transfers
RUN git config --global http.postBuffer 1048576000  # 1 GB buffer size
RUN git config --global http.maxRequestBuffer 1048576000
RUN git config --global http.lowSpeedLimit 0
RUN git config --global http.lowSpeedTime 999

# Install the risc0 toolchain using rzup
RUN curl -L https://risczero.com/install | bash && \
    chmod -R 777 /root/.risc0 && \
    /root/.risc0/bin/rzup install || true && \  
    ln -sf /root/.risc0/cargo-risczero /usr/local/bin/cargo-risczero

# Ensure the risc0 toolchain binaries are in the PATH
ENV PATH="/root/.risc0/bin:${PATH}"

# Navigate to the client directory and build the project
WORKDIR /app/client

# Build the Rust project
RUN cargo build --release

# Set the command to run your application
CMD ["cargo", "run", "--release"]
