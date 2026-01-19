# Arch Linux AMI for Jarvy E2E Testing
#
# Build with: packer build arch-linux.pkr.hcl
#
# Note: Arch Linux doesn't have official AWS AMIs.
# This uses a community AMI as base.

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
  default = "t3.medium"
}

source "amazon-ebs" "arch" {
  ami_name      = "jarvy-e2e-arch-linux-{{timestamp}}"
  instance_type = var.instance_type
  region        = var.aws_region

  source_ami_filter {
    filters = {
      name                = "arch-linux-*"
      root-device-type    = "ebs"
      virtualization-type = "hvm"
      architecture        = "x86_64"
    }
    most_recent = true
    owners      = ["647457786197"] # Arch Linux community
  }

  ssh_username = "arch"

  tags = {
    Name        = "jarvy-e2e-arch-linux"
    Project     = "jarvy"
    Component   = "e2e-testing"
    OS          = "arch-linux"
    BuildTime   = "{{timestamp}}"
  }
}

build {
  sources = ["source.amazon-ebs.arch"]

  # Update system
  provisioner "shell" {
    inline = [
      "sudo pacman -Syu --noconfirm",
      "sudo pacman -S --noconfirm base-devel git curl wget jq",
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
  provisioner "shell" {
    inline = [
      "sudo mkdir -p /opt/actions-runner",
      "sudo chown arch:arch /opt/actions-runner",
      "cd /opt/actions-runner",
      "RUNNER_VERSION=$(curl -s https://api.github.com/repos/actions/runner/releases/latest | jq -r '.tag_name' | sed 's/v//')",
      "curl -o actions-runner.tar.gz -L https://github.com/actions/runner/releases/download/v$RUNNER_VERSION/actions-runner-linux-x64-$RUNNER_VERSION.tar.gz",
      "tar xzf actions-runner.tar.gz",
      "rm actions-runner.tar.gz",
    ]
  }

  # Install AWS CLI
  provisioner "shell" {
    inline = [
      "sudo pacman -S --noconfirm aws-cli-v2",
    ]
  }

  # Clean up
  provisioner "shell" {
    inline = [
      "sudo pacman -Scc --noconfirm",
    ]
  }
}
