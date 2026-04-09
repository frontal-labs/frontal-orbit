# Safety checks to avoid conflicts with existing infrastructure

# Check if the orbit-server service exists
data "kubernetes_service" "existing_orbit_server" {
  count = var.deploy_orbit_server ? 1 : 0
  metadata {
    name      = "orbit-server"
    namespace = var.orbit_service_namespace
  }
}

# Check if the orbit-slack service exists
data "kubernetes_service" "existing_orbit_slack" {
  count = var.deploy_orbit_slack ? 1 : 0
  metadata {
    name      = "orbit-slack"
    namespace = var.orbit_service_namespace
  }
}

# Check if there's already an ingress for orbit-server
data "kubernetes_ingress_v1" "existing_orbit_ingress" {
  count = var.deploy_orbit_server ? 1 : 0
  
  metadata {
    name      = "orbit-server"
    namespace = var.orbit_service_namespace
  }
  
  depends_on = [data.kubernetes_service.existing_orbit_server]
}

# Check if namespace exists
data "kubernetes_namespace" "existing_orbit_namespace" {
  metadata {
    name = var.orbit_service_namespace
  }
}

# Validate that we're not creating conflicting Route53 records
data "aws_route53_record" "existing_tools_record" {
  zone_id = data.aws_route53_zone.frontal_dev.zone_id
  name    = "tools.frontal.dev"
  type    = "A"
}

# Check if we're in development account
data "aws_caller_identity" "current" {}

# Local values for safety checks
locals {
  # Ensure we don't create conflicts
  has_existing_orbit_server = var.deploy_orbit_server ? length(data.kubernetes_service.existing_orbit_server[0].metadata) > 0 : false
  has_existing_orbit_slack  = var.deploy_orbit_slack ? length(data.kubernetes_service.existing_orbit_slack[0].metadata) > 0 : false
  has_existing_ingress      = var.deploy_orbit_server ? length(data.kubernetes_ingress_v1.existing_orbit_ingress[0].metadata) > 0 : false
  has_existing_dns          = length(data.aws_route53_record.existing_tools_record) > 0
  has_existing_namespace    = length(data.kubernetes_namespace.existing_orbit_namespace.metadata) > 0
  
  # Account validation
  is_development_account = data.aws_caller_identity.current.account_id == var.aws_account_id
  
  # Safety validation
  safety_check_passed = (
    local.is_development_account && 
    !local.has_existing_ingress && 
    !local.has_existing_dns
  )
  
  # Warnings for existing resources
  warnings = [
    local.has_existing_orbit_server ? "Warning: orbit-server service already exists in namespace ${var.orbit_service_namespace}" : "",
    local.has_existing_orbit_slack ? "Warning: orbit-slack service already exists in namespace ${var.orbit_service_namespace}" : "",
    local.has_existing_namespace ? "Warning: namespace ${var.orbit_service_namespace} already exists" : ""
  ]
}

# Resource validation
resource "terraform_data" "safety_validation" {
  lifecycle {
    precondition {
      condition     = local.safety_check_passed
      error_message = "Safety check failed: Either not in development account or conflicting infrastructure detected. Account: ${data.aws_caller_identity.current.account_id}, Expected: ${var.aws_account_id}. Please review existing ingress and DNS records before proceeding."
    }
  }
}

# Display warnings
resource "terraform_data" "display_warnings" {
  lifecycle {
    precondition {
      condition     = true # Always run to display warnings
      error_message = join("\n", [for warning in local.warnings : warning if warning != ""])
    }
  }
}
