FROM rust:1.79 AS builder

# Configure git to use HTTPS and increase buffer size
RUN git config --global url."https://".insteadOf git:// \
    && git config --global http.postBuffer 524288000 \
    && git config --global http.lowSpeedLimit 1000 \
    && git config --global http.lowSpeedTime 300

WORKDIR /app
# Create a new workspace file specifically for the relayer build
RUN echo '[workspace]\nmembers = ["crates/relayer", "crates/common"]\nresolver = "2"\n\n[workspace.dependencies]\neyre = "0.6"\nthiserror = "2.0.3"\ntracing = "0.1.40"\ntokio = "1.41.1"\ndotenv = "0.15"\nstarknet-crypto = "0.7.3"\n' > Cargo.toml

# Copy only the needed crates and Cargo.lock
COPY Cargo.lock ./
COPY crates/relayer ./crates/relayer
COPY crates/common ./crates/common

# Build with verbose output
RUN cargo build --release -v && \
    ls -la target/release/relayer && \
    chmod +x target/release/relayer

# Use a newer Debian version that has OpenSSL 3
FROM debian:bookworm
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libssl3 \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/relayer /usr/local/bin/relayer

# Add wrapper script
RUN echo '#!/bin/sh\nwhile true; do\n  relayer\n  sleep 60\ndone' > /usr/local/bin/run-relayer.sh \
    && chmod +x /usr/local/bin/run-relayer.sh

CMD ["/usr/local/bin/run-relayer.sh"]
