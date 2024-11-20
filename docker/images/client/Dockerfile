FROM rust:latest AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    git \
    && rm -rf /var/lib/apt/lists/*

# Configure git for better reliability
RUN git config --global http.postBuffer 524288000 && \
    git config --global http.lowSpeedLimit 1000 && \
    git config --global http.lowSpeedTime 60 && \
    git config --global core.compression 0 && \
    git config --global http.sslVerify false && \
    git config --global submodule.recurse true

# Create cargo config directory and add git-fetch-with-cli config
RUN mkdir -p /usr/local/cargo/
COPY <<EOF /usr/local/cargo/config.toml
[net]
git-fetch-with-cli = true
retry = 10
EOF

# Create necessary directories with proper permissions
RUN mkdir -p /root/.risc0/bin

# Install RISC Zero tools with retry logic
RUN for i in {1..3}; do \
        curl -L https://risczero.com/install | bash && break || sleep 15; \
    done && \
    export PATH="/root/.risc0/bin:$PATH" && \
    for i in {1..3}; do \
        rzup install && break || sleep 15; \
    done && \
    for i in {1..3}; do \
        rzup install cpp && break || sleep 15; \
    done && \
    # Create symlink with exact path
    ln -sf /root/.risc0/extensions/v1.1.3-cargo-risczero/cargo-risczero /root/.risc0/bin/cargo-risczero && \
    test -f /root/.risc0/bin/cargo-risczero || (echo "RISC0 symlink creation failed" && exit 1)

WORKDIR /app

# Copy the entire project and build
COPY . .
RUN for i in {1..3}; do \
        RUST_BACKTRACE=1 cargo build --release -v --bin client && break || \
        (rm -rf ~/.cargo/registry/cache && sleep 15); \
    done && \
    test -f target/release/client || (echo "Binary not found after build" && exit 1)

# Start a fresh image
FROM rust:slim

# Copy the built binary
COPY --from=builder /app/target/release/client /usr/local/bin/

# Set the binary as the entrypoint
ENTRYPOINT ["/usr/local/bin/client"]