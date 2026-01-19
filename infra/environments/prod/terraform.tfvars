# Production Environment Variables
#
# Copy this file and fill in the values for your environment.
# Do NOT commit sensitive values to version control.

aws_region  = "us-west-2"
github_repo = "YOUR_ORG/jarvy"

# VPC and Subnet IDs
# Use your default VPC or create a dedicated one
vpc_id    = "vpc-xxxxxxxxx"
subnet_id = "subnet-xxxxxxxxx"

# AMI IDs (populated after running Packer builds)
# Run: packer build fedora-40.pkr.hcl
fedora_ami_id = ""

# Run: packer build arch-linux.pkr.hcl
arch_ami_id = ""

# Run: packer build alpine.pkr.hcl
alpine_ami_id = ""

# Run: packer build freebsd-14.pkr.hcl
freebsd_ami_id = ""
