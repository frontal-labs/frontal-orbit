output "tools_domain_url" {
  description = "URL for the tools frontend"
  value       = "https://tools.frontal.dev"
}

output "orbit_tools_url" {
  description = "URL for Orbit tools"
  value       = "https://tools.frontal.dev/orbit"
}

output "orbit_server_url" {
  description = "Internal URL for orbit-server"
  value       = var.deploy_orbit_server ? "http://orbit-server:8788" : null
}

output "orbit_slack_url" {
  description = "Internal URL for orbit-slack"
  value       = var.deploy_orbit_slack ? "http://orbit-slack:3000" : null
}

output "nginx_ingress_load_balancer" {
  description = "NLB DNS name for the NGINX ingress"
  value       = data.kubernetes_service.nginx_ingress.status.0.load_balancer.0.ingress.0.hostname
}

output "route53_record_name" {
  description = "Route53 record name"
  value       = aws_route53_record.tools_frontal_dev.name
}

output "orbit_server_deployment" {
  description = "Orbit server deployment status"
  value       = var.deploy_orbit_server ? "Deployed with ${length(kubernetes_deployment.orbit_server[0].spec.replica)} replicas" : "Not deployed"
}

output "orbit_slack_deployment" {
  description = "Orbit Slack deployment status"
  value       = var.deploy_orbit_slack ? "Deployed with ${length(kubernetes_deployment.orbit_slack[0].spec.replica)} replicas" : "Not deployed"
}

output "storage_info" {
  description = "Storage configuration"
  value = var.deploy_orbit_server ? {
    workspace_size    = var.workspace_storage_size
    server_state_size = var.server_state_storage_size
    agent_store_size  = var.agent_store_storage_size
    storage_class     = var.storage_class
  } : null
}

output "environment_info" {
  description = "Environment configuration"
  value = {
    environment    = var.environment
    aws_region     = var.aws_region
    aws_account_id = var.aws_account_id
    namespace      = var.orbit_service_namespace
  }
}

output "ecr_repositories" {
  description = "ECR repositories created"
  value = {
    orbit_server = var.deploy_orbit_server && length(regexall("amazonaws\\.com", var.orbit_server_image)) > 0 ? aws_ecr_repository.orbit_server[0].repository_url : null
    orbit_slack  = var.deploy_orbit_slack && length(regexall("amazonaws\\.com", var.orbit_slack_image)) > 0 ? aws_ecr_repository.orbit_slack[0].repository_url : null
  }
}
