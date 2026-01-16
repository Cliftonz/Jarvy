# Fedora test container for Jarvy integration tests
#
# Usage:
#   docker build -f tests/docker/fedora.dockerfile -t jarvy-test-fedora .
#   docker run --rm jarvy-test-fedora cargo test --test tools_install
#
FROM fedora:39

# Install system dependencies
RUN dnf install -y \
    curl \
    wget \
    git \
    gcc \
    make \
    openssl-devel \
    pkgconf-pkg-config \
    ca-certificates \
    sudo \
    && dnf clean all

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
