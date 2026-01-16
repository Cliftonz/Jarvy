# Alpine test container for Jarvy integration tests
#
# Usage:
#   docker build -f tests/docker/alpine.dockerfile -t jarvy-test-alpine .
#   docker run --rm jarvy-test-alpine cargo test --test tools_install
#
FROM alpine:3.19

# Install system dependencies
RUN apk add --no-cache \
    curl \
    wget \
    git \
    build-base \
    openssl-dev \
    pkgconf \
    ca-certificates \
    sudo \
    bash

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Create a non-root user for testing
RUN adduser -D -s /bin/bash testuser \
    && echo "testuser ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Set working directory
WORKDIR /workspace

# Copy project files
COPY . .

# Build the project (use musl target)
RUN cargo build --release

# Set test environment variables
ENV JARVY_TEST_MODE=1
ENV RUST_BACKTRACE=1

# Default command: run integration tests
CMD ["cargo", "test", "--test", "tools_install", "--", "--show-output"]
