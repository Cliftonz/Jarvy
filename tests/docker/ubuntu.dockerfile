# Ubuntu test container for Jarvy integration tests
#
# Usage:
#   docker build -f tests/docker/ubuntu.dockerfile -t jarvy-test-ubuntu .
#   docker run --rm jarvy-test-ubuntu cargo test --test tools_install
#
FROM ubuntu:22.04

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    sudo \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Create a non-root user for testing
RUN useradd -m -s /bin/bash testuser \
    && echo "testuser ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Set working directory
WORKDIR /workspace

# Copy project files
COPY . .

# Build the project
RUN cargo build --release

# Set test environment variables
ENV JARVY_TEST_MODE=1
ENV RUST_BACKTRACE=1

# Default command: run integration tests
CMD ["cargo", "test", "--test", "tools_install", "--", "--show-output"]
