# EC2 Self-Hosted Runner Module
#
# Creates an ephemeral EC2 Spot instance that registers as a GitHub Actions
# self-hosted runner, executes a single job, then terminates.

terraform {
  required_version = ">= 1.5"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

# Data sources
data "aws_region" "current" {}

data "aws_caller_identity" "current" {}

# IAM Role for the runner
resource "aws_iam_role" "runner" {
  name = "jarvy-e2e-runner-${var.platform}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })

  tags = var.tags
}

resource "aws_iam_role_policy" "runner" {
  name = "jarvy-e2e-runner-policy"
  role = aws_iam_role.runner.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        # Allow instance to terminate itself
        Effect = "Allow"
        Action = [
          "ec2:TerminateInstances"
        ]
        Resource = "*"
        Condition = {
          StringEquals = {
            "ec2:ResourceTag/Purpose" = "jarvy-e2e-runner"
          }
        }
      },
      {
        # CloudWatch Logs for debugging
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents"
        ]
        Resource = "arn:aws:logs:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:log-group:/jarvy/e2e-runners/*"
      }
    ]
  })
}

resource "aws_iam_instance_profile" "runner" {
  name = "jarvy-e2e-runner-${var.platform}"
  role = aws_iam_role.runner.name
}

# Security Group
resource "aws_security_group" "runner" {
  name        = "jarvy-e2e-runner-${var.platform}"
  description = "Security group for Jarvy E2E runner"
  vpc_id      = var.vpc_id

  # Outbound: Allow all (needed for package downloads)
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # No inbound rules - runner only needs outbound access

  tags = merge(var.tags, {
    Name = "jarvy-e2e-runner-${var.platform}"
  })
}

# Launch Template for Spot Instances
resource "aws_launch_template" "runner" {
  name = "jarvy-e2e-runner-${var.platform}"

  image_id      = var.ami_id
  instance_type = var.instance_type

  iam_instance_profile {
    arn = aws_iam_instance_profile.runner.arn
  }

  vpc_security_group_ids = [aws_security_group.runner.id]

  # Use Spot instances for cost savings
  instance_market_options {
    market_type = "spot"
    spot_options {
      max_price                      = var.spot_max_price
      instance_interruption_behavior = "terminate"
    }
  }

  # User data script to register as GitHub runner
  user_data = base64encode(templatefile("${path.module}/user-data.sh", {
    github_repo   = var.github_repo
    runner_labels = var.runner_labels
    platform      = var.platform
  }))

  tag_specifications {
    resource_type = "instance"
    tags = merge(var.tags, {
      Name    = "jarvy-e2e-runner-${var.platform}"
      Purpose = "jarvy-e2e-runner"
    })
  }

  tag_specifications {
    resource_type = "volume"
    tags = merge(var.tags, {
      Name = "jarvy-e2e-runner-${var.platform}"
    })
  }

  tags = var.tags
}

# Note: Actual instance creation is triggered by GitHub Actions workflow
# via AWS CLI or a Lambda function. This module just defines the template.
