# Outputs for EC2 Self-Hosted Runner Module

output "launch_template_id" {
  description = "ID of the launch template"
  value       = aws_launch_template.runner.id
}

output "launch_template_latest_version" {
  description = "Latest version of the launch template"
  value       = aws_launch_template.runner.latest_version
}

output "security_group_id" {
  description = "ID of the security group"
  value       = aws_security_group.runner.id
}

output "iam_role_arn" {
  description = "ARN of the IAM role"
  value       = aws_iam_role.runner.arn
}

output "instance_profile_name" {
  description = "Name of the IAM instance profile"
  value       = aws_iam_instance_profile.runner.name
}
