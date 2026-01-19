# Alpine Linux AMI for Jarvy E2E Testing
#
# Build with: packer build alpine.pkr.hcl

packer {
  required_plugins {
    amazon = {
      version = ">= 1.2.0"
      source  = "github.com/hashicorp/amazon"
    }
  }
}

variable "aws_region" {
  type    = string
  default = "us-west-2"
}

variable "instance_type" {
  type    = string
  default = "t3.small"
}

source "amazon-ebs" "alpine" {
  ami_name      = "jarvy-e2e-alpine-{{timestamp}}"
  instance_type = var.instance_type
  region        = var.aws_region

  source_ami_filter {
    filters = {
      name                = "alpine-*-x86_64-*"
      root-device-type    = "ebs"
      virtualization-type = "hvm"
      architecture        = "x86_64"
    }
    most_recent = true
    owners      = ["538276064493"] # Alpine community
  }

  ssh_username = "alpine"

  tags = {
    Name        = "jarvy-e2e-alpine"
    Project     = "jarvy"
    Component   = "e2e-testing"
    OS          = "alpine"
    BuildTime   = "{{timestamp}}"
  }
}

build {
  sources = ["source.amazon-ebs.alpine"]

  # Update system and install dependencies
  provisioner "shell" {
    inline = [
      "sudo apk update",
      "sudo apk upgrade",
      "sudo apk add build-base git curl wget jq bash",
    ]
  }

  # Install Rust
  provisioner "shell" {
    inline = [
      "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
      "source $HOME/.cargo/env",
      "rustup default stable",
      "rustc --version",
    ]
  }

  # Pre-download GitHub Actions runner
  # Note: Alpine uses musl, need to check runner compatibility
  provisioner "shell" {
    inline = [
      "sudo mkdir -p /opt/actions-runner",
      "sudo chown alpine:alpine /opt/actions-runner",
      "cd /opt/actions-runner",
      "RUNNER_VERSION=$(curl -s https://api.github.com/repos/actions/runner/releases/latest | jq -r '.tag_name' | sed 's/v//')",
      "curl -o actions-runner.tar.gz -L https://github.com/actions/runner/releases/download/v$RUNNER_VERSION/actions-runner-linux-x64-$RUNNER_VERSION.tar.gz",
      "tar xzf actions-runner.tar.gz",
      "rm actions-runner.tar.gz",
      # Install glibc compatibility layer for GitHub runner
      "sudo apk add gcompat libstdc++",
    ]
  }

  # Install AWS CLI
  provisioner "shell" {
    inline = [
      "sudo apk add aws-cli",
    ]
  }

  # Clean up
  provisioner "shell" {
    inline = [
      "sudo rm -rf /var/cache/apk/*",
    ]
  }
}
