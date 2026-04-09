# Optional: Build and push Docker images using Terraform
# This is useful for CI/CD pipelines

resource "terraform_data" "build_orbit_server_image" {
  count = var.deploy_orbit_server ? 1 : 0
  
  triggers_replace = [
    var.orbit_server_image
  ]

  provisioner "local-exec" {
    command = <<-EOT
      echo "Building orbit-server image: ${var.orbit_server_image}"
      
      # Build the image
      docker build -f infrastructure/docker/orbit-server.Dockerfile -t ${var.orbit_server_image} .
      
      # Tag for ECR if needed
      if [[ "${var.orbit_server_image}" == *"amazonaws.com"* ]]; then
        # Extract ECR details
        ECR_REGISTRY=$(echo "${var.orbit_server_image}" | cut -d'/' -f1)
        REPOSITORY_NAME=$(echo "${var.orbit_server_image}" | cut -d'/' -f2 | cut -d':' -f1)
        
        # Login to ECR
        aws ecr get-login-password --region ${var.aws_region} | docker login --username AWS --password-stdin $ECR_REGISTRY
        
        # Create repository if it doesn't exist
        aws ecr describe-repositories --repository-names $REPOSITORY_NAME --region ${var.aws_region} || \
          aws ecr create-repository --repository-name $REPOSITORY_NAME --region ${var.aws_region}
        
        # Push the image
        docker push ${var.orbit_server_image}
      fi
    EOT
    
    working_dir = path.module
  }
}

resource "terraform_data" "build_orbit_slack_image" {
  count = var.deploy_orbit_slack ? 1 : 0
  
  triggers_replace = [
    var.orbit_slack_image
  ]

  provisioner "local-exec" {
    command = <<-EOT
      echo "Building orbit-slack image: ${var.orbit_slack_image}"
      
      # Build the image
      docker build -f extensions/orbit-slack/Dockerfile -t ${var.orbit_slack_image} extensions/orbit-slack/
      
      # Tag for ECR if needed
      if [[ "${var.orbit_slack_image}" == *"amazonaws.com"* ]]; then
        # Extract ECR details
        ECR_REGISTRY=$(echo "${var.orbit_slack_image}" | cut -d'/' -f1)
        REPOSITORY_NAME=$(echo "${var.orbit_slack_image}" | cut -d'/' -f2 | cut -d':' -f1)
        
        # Login to ECR
        aws ecr get-login-password --region ${var.aws_region} | docker login --username AWS --password-stdin $ECR_REGISTRY
        
        # Create repository if it doesn't exist
        aws ecr describe-repositories --repository-names $REPOSITORY_NAME --region ${var.aws_region} || \
          aws ecr create-repository --repository-name $REPOSITORY_NAME --region ${var.aws_region}
        
        # Push the image
        docker push ${var.orbit_slack_image}
      fi
    EOT
    
    working_dir = path.module
  }
}

# ECR Repository resources (optional - for managed repositories)
resource "aws_ecr_repository" "orbit_server" {
  count = var.deploy_orbit_server && var.orbit_server_image != "orbit-server:latest" ? 1 : 0
  name                 = "orbit-server"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }

  tags = {
    Environment = var.environment
    Project     = "orbit-tools"
  }
}

resource "aws_ecr_repository" "orbit_slack" {
  count = var.deploy_orbit_slack && var.orbit_slack_image != "orbit-slack:latest" ? 1 : 0
  name                 = "orbit-slack"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }

  tags = {
    Environment = var.environment
    Project     = "orbit-tools"
  }
}

# ECR Lifecycle policies
resource "aws_ecr_lifecycle_policy" "orbit_server" {
  count = var.deploy_orbit_server && var.orbit_server_image != "orbit-server:latest" ? 1 : 0
  repository = aws_ecr_repository.orbit_server[0].name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last 30 images"
        selection = {
          tagStatus     = "tagged"
          tagPrefixList = ["v"]
          countType     = "imageCountMoreThan"
          countNumber   = 30
        }
        action = {
          type = "expire"
        }
      },
      {
        rulePriority = 2
        description  = "Keep last 10 untagged images"
        selection = {
          tagStatus = "untagged"
          countType = "imageCountMoreThan"
          countNumber = 10
        }
        action = {
          type = "expire"
        }
      }
    ]
  })
}

resource "aws_ecr_lifecycle_policy" "orbit_slack" {
  count = var.deploy_orbit_slack && var.orbit_slack_image != "orbit-slack:latest" ? 1 : 0
  repository = aws_ecr_repository.orbit_slack[0].name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last 30 images"
        selection = {
          tagStatus     = "tagged"
          tagPrefixList = ["v"]
          countType     = "imageCountMoreThan"
          countNumber   = 30
        }
        action = {
          type = "expire"
        }
      },
      {
        rulePriority = 2
        description  = "Keep last 10 untagged images"
        selection = {
          tagStatus = "untagged"
          countType = "imageCountMoreThan"
          countNumber = 10
        }
        action = {
          type = "expire"
        }
      }
    ]
  })
}
