terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.20"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.10"
    }
  }
}

provider "aws" {
  region = var.aws_region
  
  default_tags {
    tags = {
      Project     = "orbit-tools"
      Environment = var.environment
      ManagedBy   = "terraform"
    }
  }
}

data "aws_route53_zone" "frontal_dev" {
  name = "frontal.dev."
}

data "aws_eks_cluster" "orbit" {
  name = var.eks_cluster_name
}

data "aws_eks_cluster_auth" "orbit" {
  name = var.eks_cluster_auth_name
}

provider "kubernetes" {
  host                   = data.aws_eks_cluster.orbit.endpoint
  cluster_ca_certificate = base64decode(data.aws_eks_cluster.orbit.certificate_authority[0].data)
  token                  = data.aws_eks_cluster_auth.orbit.token
}

provider "helm" {
  kubernetes {
    host                   = data.aws_eks_cluster.orbit.endpoint
    cluster_ca_certificate = base64decode(data.aws_eks_cluster.orbit.certificate_authority[0].data)
    token                  = data.aws_eks_cluster_auth.orbit.token
  }
}
