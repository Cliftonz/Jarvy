# FreeBSD 14 AMI for Jarvy E2E Testing
#
# Build with: packer build freebsd-14.pkr.hcl

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

source "amazon-ebs" "freebsd" {
  ami_name      = "jarvy-e2e-freebsd-14-{{timestamp}}"
  instance_type = var.instance_type
  region        = var.aws_region

  source_ami_filter {
    filters = {
      name                = "FreeBSD 14.*-RELEASE*"
      root-device-type    = "ebs"
      virtualization-type = "hvm"
      architecture        = "x86_64"
    }
    most_recent = true
    owners      = ["782442783595"] # FreeBSD official
  }

  ssh_username = "ec2-user"

  tags = {
    Name        = "jarvy-e2e-freebsd-14"
    Project     = "jarvy"
    Component   = "e2e-testing"
    OS          = "freebsd-14"
    BuildTime   = "{{timestamp}}"
  }
}

build {
  sources = ["source.amazon-ebs.freebsd"]

  # Update system and install dependencies
  provisioner "shell" {
    inline = [
      "sudo pkg update",
      "sudo pkg upgrade -y",
      "sudo pkg install -y git curl wget jq bash",
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

  # Note: GitHub Actions runner doesn't officially support FreeBSD
  # We'll need to use a custom runner solution or skip FreeBSD in CI
  provisioner "shell" {
    inline = [
      "echo 'Note: GitHub Actions runner does not officially support FreeBSD'",
      "echo 'Consider using a custom runner implementation for FreeBSD testing'",
    ]
  }

  # Install AWS CLI
  provisioner "shell" {
    inline = [
      "sudo pkg install -y awscli",
    ]
  }

  # Clean up
  provisioner "shell" {
    inline = [
      "sudo pkg clean -y",
    ]
  }
}
