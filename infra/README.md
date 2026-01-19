# Jarvy E2E Testing Infrastructure

This directory contains infrastructure-as-code for the E2E testing harness described in PRD-038.

## Architecture

The E2E testing uses a **hybrid approach**:

1. **GitHub-hosted runners (FREE)**: macOS, Ubuntu, Windows
2. **AWS EC2 Spot instances (~$0.01/run)**: Fedora, Arch, Alpine, FreeBSD

## Directory Structure

```
infra/
├── README.md                    # This file
├── modules/
│   └── ec2-runner/              # Terraform module for ephemeral runners
│       ├── main.tf              # EC2 instance, security group, IAM
│       ├── variables.tf         # Input variables
│       ├── outputs.tf           # Outputs
│       └── user-data.sh         # Bootstrap script
├── packer/                      # AMI build definitions
│   ├── fedora-40.pkr.hcl
│   ├── arch-linux.pkr.hcl
│   ├── alpine.pkr.hcl
│   └── freebsd-14.pkr.hcl
└── environments/
    └── prod/                    # Production environment
        ├── main.tf
        └── terraform.tfvars
```

## Prerequisites

- AWS CLI configured with appropriate credentials
- Terraform >= 1.5
- Packer >= 1.9 (for building AMIs)

## Usage

### 1. Build Custom AMIs

```bash
cd packer
packer build fedora-40.pkr.hcl
packer build arch-linux.pkr.hcl
packer build alpine.pkr.hcl
packer build freebsd-14.pkr.hcl
```

### 2. Deploy Infrastructure

```bash
cd environments/prod
terraform init
terraform plan
terraform apply
```

### 3. Configure GitHub Secrets

Add these secrets to your GitHub repository:

- `AWS_ACCESS_KEY_ID`: AWS access key for runner provisioning
- `AWS_SECRET_ACCESS_KEY`: AWS secret key
- `GH_RUNNER_TOKEN`: GitHub runner registration token

## Cost Estimate

| Component | Per Run | Monthly (100 runs) |
|-----------|---------|-------------------|
| Fedora (t3.medium, 20 min) | $0.003 | $0.30 |
| Arch (t3.medium, 20 min) | $0.003 | $0.30 |
| Alpine (t3.small, 15 min) | $0.001 | $0.10 |
| FreeBSD (t3.medium, 20 min) | $0.003 | $0.30 |
| **Total** | **~$0.01** | **~$1.00** |

## Security

- Ephemeral runners terminate after single job
- IAM role with minimal permissions
- VPC isolation with NAT gateway for egress
- No secrets stored on runner instances
