# Fedora 40 AMI for Jarvy E2E Testing
#
# Build with: packer build fedora-40.pkr.hcl

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

source "amazon-ebs" "fedora" {
  ami_name      = "jarvy-e2e-fedora-40-{{timestamp}}"
  instance_type = var.instance_type
  region        = var.aws_region

  source_ami_filter {
    filters = {
      name                = "Fedora-Cloud-Base-40-*"
      root-device-type    = "ebs"
      virtualization-type = "hvm"
      architecture        = "x86_64"
    }
    most_recent = true
    owners      = ["125523088429"] # Fedora official
  }

  ssh_username = "fedora"

  tags = {
    Name        = "jarvy-e2e-fedora-40"
    Project     = "jarvy"
    Component   = "e2e-testing"
    OS          = "fedora-40"
    BuildTime   = "{{timestamp}}"
  }
}

build {
  sources = ["source.amazon-ebs.fedora"]

  # Update system
  provisioner "shell" {
    inline = [
      "sudo dnf update -y",
      "sudo dnf install -y @development-tools git curl wget jq",
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
      "sudo chown fedora:fedora /opt/actions-runner",
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
      "sudo dnf install -y awscli2",
    ]
  }

  # Clean up
  provisioner "shell" {
    inline = [
      "sudo dnf clean all",
      "sudo rm -rf /var/cache/dnf/*",
    ]
  }
}
