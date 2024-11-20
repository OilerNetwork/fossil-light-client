FROM ubuntu:22.04

# Install basic dependencies
RUN apt-get update && \
    apt-get install -y curl ca-certificates git build-essential && \
    rm -rf /var/lib/apt/lists/*

# Set up PATH environment variable
ENV PATH="/root/.local/bin:/root/.dojo/bin:/root/.cargo/bin:$PATH"

# Create necessary directories
RUN mkdir -p /root/.local/bin

# Install Dojo and Katana
RUN curl -L https://install.dojoengine.org | bash && \
    . /root/.bashrc && \
    dojoup -v 1.0.0-alpha.16

# Install Scarb with proper PATH setup
RUN curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | bash && \
    echo 'export PATH="/root/.local/bin:$PATH"' >> /root/.bashrc && \
    . /root/.bashrc

# Source the environment and verify installations
RUN . /root/.bashrc && \
    echo "Testing Scarb installation:" && scarb --version && \
    echo "Testing Katana installation:" && katana --version

EXPOSE 5050

WORKDIR /app

# Add environment setup to every shell session
RUN echo '. /root/.bashrc' >> /root/.profile
# SHELL ["/bin/bash", "-c"]