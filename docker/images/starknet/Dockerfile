FROM ubuntu:22.04

# Install necessary tools
RUN apt-get update && \
    apt-get install -y \
    curl \
    ca-certificates \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Configure git to allow cloning from HTTPS
RUN git config --global credential.helper store && \
    git config --global url."https://".insteadOf git:// && \
    git config --global http.sslVerify false

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Foundry
RUN curl -L https://foundry.paradigm.xyz | bash && \
    /root/.foundry/bin/foundryup
ENV PATH="/root/.foundry/bin:${PATH}"

# Install Scarb manually
RUN mkdir -p /root/.local/bin && \
    mkdir -p /root/.local/share/scarb-install/latest && \
    curl -L https://github.com/software-mansion/scarb/releases/download/v2.8.4/scarb-v2.8.4-x86_64-unknown-linux-gnu.tar.gz -o scarb.tar.gz && \
    tar -xzf scarb.tar.gz -C /root/.local/share/scarb-install/latest --strip-components=1 && \
    ln -s /root/.local/share/scarb-install/latest/bin/scarb /root/.local/bin/scarb && \
    rm scarb.tar.gz

# Set up PATH
ENV PATH="/root/.local/bin:${PATH}"

# Install starkli manually
RUN mkdir -p /root/.starkli/bin && \
    curl -L https://github.com/xJonathanLEI/starkli/releases/download/v0.3.5/starkli-x86_64-unknown-linux-gnu.tar.gz -o starkli.tar.gz && \
    tar -xzf starkli.tar.gz -C /root/.starkli/bin && \
    rm starkli.tar.gz && \
    chmod +x /root/.starkli/bin/starkli

# Add starkli to PATH
ENV PATH="/root/.starkli/bin:${PATH}"

# Set up PATH for all tools
ENV PATH="/root/.foundry/bin:/root/.local/bin:/root/.dojo/bin:/root/.cargo/bin:${PATH}"

# Verify installations
RUN forge --version && \
    cast --version && \
    scarb --version && \
    starkli --version

# Create and set permissions for Scarb cache directory
RUN mkdir -p /root/.cache/scarb && \
    chmod -R 777 /root/.cache/scarb

WORKDIR /app

# Copy scripts directory first and verify it exists
COPY scripts/katana/deploy.sh /app/scripts/katana/
RUN ls -la /app/scripts/katana/deploy.sh || echo "Script not found!"

# Make the deployment script executable
RUN chmod +x /app/scripts/katana/deploy.sh

# Copy the rest of the project
COPY . .

# Debug: List contents of relevant directories
RUN echo "Listing /app directory:" && \
    ls -la /app && \
    echo "Listing /app/scripts directory:" && \
    ls -la /app/scripts && \
    echo "Listing /app/scripts/katana directory:" && \
    ls -la /app/scripts/katana

# Initialize directories for Starkli
RUN mkdir -p /root/.starkli/accounts && \
    mkdir -p /root/.starkli/keystores

# Try using bash to execute the script
ENTRYPOINT ["/bin/bash", "/app/scripts/katana/deploy.sh"]
