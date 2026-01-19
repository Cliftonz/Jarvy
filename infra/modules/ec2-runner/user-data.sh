#!/bin/bash
# Jarvy E2E Runner Bootstrap Script
#
# This script:
# 1. Downloads and configures GitHub Actions runner
# 2. Registers as an ephemeral runner (auto-removes after one job)
# 3. Starts the runner
# 4. Terminates the instance when the job completes

set -euo pipefail

# Configuration (injected by Terraform)
GITHUB_REPO="${github_repo}"
RUNNER_LABELS="${runner_labels}"
PLATFORM="${platform}"

# Logging
exec > >(tee /var/log/runner-bootstrap.log) 2>&1
echo "Starting runner bootstrap at $(date)"
echo "Platform: $PLATFORM"
echo "Repository: $GITHUB_REPO"

# Wait for cloud-init to complete
cloud-init status --wait || true

# Create runner directory
RUNNER_DIR="/opt/actions-runner"
mkdir -p "$RUNNER_DIR"
cd "$RUNNER_DIR"

# Determine architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64) RUNNER_ARCH="x64" ;;
    aarch64) RUNNER_ARCH="arm64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Get latest runner version
RUNNER_VERSION=$(curl -s https://api.github.com/repos/actions/runner/releases/latest | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')
echo "Installing GitHub Actions Runner v$RUNNER_VERSION"

# Download runner
RUNNER_URL="https://github.com/actions/runner/releases/download/v$${RUNNER_VERSION}/actions-runner-linux-$${RUNNER_ARCH}-$${RUNNER_VERSION}.tar.gz"
curl -o actions-runner.tar.gz -L "$RUNNER_URL"
tar xzf actions-runner.tar.gz
rm actions-runner.tar.gz

# Get runner registration token
# Note: In production, this should come from AWS Secrets Manager or Parameter Store
# For now, we expect it to be passed via instance metadata or environment
if [ -z "$${RUNNER_TOKEN:-}" ]; then
    # Try to get from instance metadata (set by workflow)
    RUNNER_TOKEN=$(curl -s http://169.254.169.254/latest/meta-data/tags/instance/RunnerToken || echo "")
fi

if [ -z "$RUNNER_TOKEN" ]; then
    echo "ERROR: No runner token available. Cannot register runner."
    exit 1
fi

# Configure runner as ephemeral (removes itself after one job)
./config.sh \
    --url "https://github.com/$GITHUB_REPO" \
    --token "$RUNNER_TOKEN" \
    --labels "$RUNNER_LABELS" \
    --name "jarvy-e2e-$PLATFORM-$(hostname)" \
    --ephemeral \
    --unattended \
    --replace

echo "Runner configured successfully"

# Install runner service
./svc.sh install || true

# Start the runner
echo "Starting runner..."
./run.sh &
RUNNER_PID=$!

# Wait for runner to complete (ephemeral runner exits after one job)
wait $RUNNER_PID
EXIT_CODE=$?

echo "Runner exited with code $EXIT_CODE"

# Self-terminate the instance
echo "Terminating instance..."
INSTANCE_ID=$(curl -s http://169.254.169.254/latest/meta-data/instance-id)
aws ec2 terminate-instances --instance-ids "$INSTANCE_ID" --region $(curl -s http://169.254.169.254/latest/meta-data/placement/region)

exit $EXIT_CODE
