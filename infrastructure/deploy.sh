#!/bin/bash

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERRAFORM_DIR="${SCRIPT_DIR}/terraform"

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if terraform is installed
    if ! command -v terraform &> /dev/null; then
        log_error "Terraform is not installed. Please install Terraform first."
        exit 1
    fi
    
    # Check if kubectl is installed
    if ! command -v kubectl &> /dev/null; then
        log_error "kubectl is not installed. Please install kubectl first."
        exit 1
    fi
    
    # Check if AWS CLI is installed
    if ! command -v aws &> /dev/null; then
        log_error "AWS CLI is not installed. Please install AWS CLI first."
        exit 1
    fi
    
    # Check if jq is installed (for JSON parsing)
    if ! command -v jq &> /dev/null; then
        log_error "jq is not installed. Please install jq for JSON parsing."
        exit 1
    fi
    
    # Check if we can connect to the cluster
    if ! kubectl cluster-info &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster. Please check your kubeconfig."
        exit 1
    fi
    
    log_success "All prerequisites met"
}

# Initialize Terraform
init_terraform() {
    log_info "Initializing Terraform..."
    
    cd "$TERRAFORM_DIR"
    
    if [ ! -f "terraform.tfvars" ]; then
        log_warning "terraform.tfvars not found. Copying from example..."
        cp terraform.tfvars.example terraform.tfvars
        log_warning "Please edit terraform.tfvars with your configuration before proceeding."
        exit 1
    fi
    
    terraform init
    
    log_success "Terraform initialized"
}

# Plan Terraform
plan_terraform() {
    log_info "Planning Terraform changes..."
    
    cd "$TERRAFORM_DIR"
    terraform plan
    
    log_success "Terraform plan completed"
}

# Apply Terraform
apply_terraform() {
    log_info "Applying Terraform changes..."
    
    cd "$TERRAFORM_DIR"
    terraform apply -auto-approve
    
    log_success "Terraform applied successfully"
}

# Verify deployment
verify_deployment() {
    log_info "Verifying deployment..."
    
    cd "$TERRAFORM_DIR"
    
    # Get outputs
    TOOLS_URL=$(terraform output -raw tools_domain_url)
    ORBIT_URL=$(terraform output -raw orbit_tools_url)
    NLB_DNS=$(terraform output -raw nginx_ingress_load_balancer)
    
    log_info "Waiting for DNS propagation..."
    sleep 30
    
    # Check DNS resolution
    if nslookup tools.frontal.dev &> /dev/null; then
        log_success "DNS is resolving correctly"
    else
        log_warning "DNS may still be propagating. Please check again in a few minutes."
    fi
    
    # Check Kubernetes resources
    log_info "Checking Kubernetes resources..."
    
    if kubectl get ingress -n orbit orbit-tools-ingress &> /dev/null; then
        log_success "Ingress created successfully"
    else
        log_error "Ingress not found"
        return 1
    fi
    
    if kubectl get pods -n orbit -l app=tools-proxy &> /dev/null; then
        log_success "Proxy pods are running"
    else
        log_error "Proxy pods not found"
        return 1
    fi
    
    log_success "Deployment verification completed"
}

# Show results
show_results() {
    cd "$TERRAFORM_DIR"
    
    TOOLS_URL=$(terraform output -raw tools_domain_url)
    ORBIT_URL=$(terraform output -raw orbit_tools_url)
    NLB_DNS=$(terraform output -raw nginx_ingress_load_balancer)
    ORBIT_SERVER_DEPLOYMENT=$(terraform output -raw orbit_server_deployment)
    ORBIT_SLACK_DEPLOYMENT=$(terraform output -raw orbit_slack_deployment)
    ENVIRONMENT_INFO=$(terraform output -json environment_info)
    
    echo
    log_success "=== Deployment Complete ==="
    echo
    echo "Environment: $(echo "$ENVIRONMENT_INFO" | jq -r '.environment') ($(echo "$ENVIRONMENT_INFO" | jq -r '.aws_region'))"
    echo "AWS Account: $(echo "$ENVIRONMENT_INFO" | jq -r '.aws_account_id')"
    echo "Namespace: $(echo "$ENVIRONMENT_INFO" | jq -r '.namespace')"
    echo
    echo "URLs:"
    echo "  Tools Landing Page: $TOOLS_URL"
    echo "  Orbit API Endpoint: $ORBIT_URL"
    echo "  NLB DNS Name: $NLB_DNS"
    echo
    echo "Deployments:"
    echo "  Orbit Server: $ORBIT_SERVER_DEPLOYMENT"
    echo "  Orbit Slack: $ORBIT_SLACK_DEPLOYMENT"
    echo
    
    # Show storage info if orbit-server is deployed
    if terraform output -raw orbit_server_deployment | grep -q "Deployed"; then
        STORAGE_INFO=$(terraform output -json storage_info)
        echo "Storage Configuration:"
        echo "  Workspace: $(echo "$STORAGE_INFO" | jq -r '.workspace_size')"
        echo "  Server State: $(echo "$STORAGE_INFO" | jq -r '.server_state_size')"
        echo "  Agent Store: $(echo "$STORAGE_INFO" | jq -r '.agent_store_size')"
        echo "  Storage Class: $(echo "$STORAGE_INFO" | jq -r '.storage_class')"
        echo
    fi
    
    # Show ECR repositories if created
    ECR_REPOS=$(terraform output -json ecr_repositories)
    if [[ "$(echo "$ECR_REPOS" | jq -r '.orbit_server')" != "null" ]]; then
        echo "ECR Repositories:"
        echo "  Orbit Server: $(echo "$ECR_REPOS" | jq -r '.orbit_server')"
        echo "  Orbit Slack: $(echo "$ECR_REPOS" | jq -r '.orbit_slack')"
        echo
    fi
    
    echo "Environment variables for Orbit CLI:"
    echo "export FRONTAL_BASE_URL=\"$ORBIT_URL\""
    echo "export ORBIT_HOSTED_CALLBACK_URL=\"$ORBIT_URL/webhooks/tasks\""
    echo
    echo "Next steps:"
    echo "1. Update your Orbit CLI configuration with the URLs above"
    echo "2. Test the deployment by visiting: $TOOLS_URL"
    echo "3. Check the Orbit API health: $ORBIT_URL/health"
    echo "4. Verify Slack integration is working (if deployed)"
    echo "5. Check Kubernetes resources:"
    echo "   kubectl get pods -n $(echo "$ENVIRONMENT_INFO" | jq -r '.namespace')"
    echo "   kubectl get services -n $(echo "$ENVIRONMENT_INFO" | jq -r '.namespace')"
    echo "   kubectl get ingress -n $(echo "$ENVIRONMENT_INFO" | jq -r '.namespace')"
    echo
}

# Cleanup function
cleanup() {
    if [ $? -ne 0 ]; then
        log_error "Deployment failed. Please check the error messages above."
        exit 1
    fi
}

# Main execution
main() {
    echo "=== tools.frontal.dev Deployment Script ==="
    echo
    
    trap cleanup EXIT
    
    check_prerequisites
    init_terraform
    
    # Ask user if they want to see the plan first
    echo
    read -p "Do you want to see the Terraform plan first? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        plan_terraform
        echo
        read -p "Continue with apply? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Deployment cancelled by user."
            exit 0
        fi
    fi
    
    apply_terraform
    verify_deployment
    show_results
    
    log_success "Deployment completed successfully!"
}

# Handle script arguments
case "${1:-}" in
    "plan")
        check_prerequisites
        init_terraform
        plan_terraform
        ;;
    "apply")
        check_prerequisites
        init_terraform
        apply_terraform
        verify_deployment
        show_results
        ;;
    "destroy")
        log_warning "This will destroy all infrastructure created by this script."
        read -p "Are you sure? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            cd "$TERRAFORM_DIR"
            terraform destroy -auto-approve
        else
            log_info "Destroy cancelled by user."
        fi
        ;;
    "help"|"-h"|"--help")
        echo "Usage: $0 [command]"
        echo
        echo "Commands:"
        echo "  plan    - Show Terraform plan without applying"
        echo "  apply   - Apply Terraform changes (default)"
        echo "  destroy - Destroy all created infrastructure"
        echo "  help    - Show this help message"
        echo
        ;;
    "")
        main
        ;;
    *)
        log_error "Unknown command: $1"
        echo "Use '$0 help' for usage information."
        exit 1
        ;;
esac
