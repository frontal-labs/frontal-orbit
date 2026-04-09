# Install NGINX Ingress Controller if not already present
resource "helm_release" "nginx_ingress" {
  name       = "nginx-ingress-controller"
  repository = "https://kubernetes.github.io/ingress-nginx"
  chart      = "ingress-nginx"
  namespace  = "ingress-nginx"
  version    = "4.8.3"

  create_namespace = true

  values = [
    yamlencode({
      controller = {
        replicaCount = 2
        nodeSelector = {
          "kubernetes.io/os" = "linux"
        }
        affinity = {
          podAntiAffinity = {
            preferredDuringSchedulingIgnoredDuringExecution = [
              {
                weight = 100
                podAffinityTerm = {
                  labelSelector = {
                    matchLabels = {
                      "app.kubernetes.io/name"      = "ingress-nginx"
                      "app.kubernetes.io/component" = "controller"
                    }
                  }
                  topologyKey = "kubernetes.io/hostname"
                }
              }
            ]
          }
        }
        resources = {
          limits = {
            cpu    = "500m"
            memory = "512Mi"
          }
          requests = {
            cpu    = "250m"
            memory = "256Mi"
          }
        }
        service = {
          type = "LoadBalancer"
          annotations = {
            "service.beta.kubernetes.io/aws-load-balancer-type"                              = "nlb"
            "service.beta.kubernetes.io/aws-load-balancer-scheme"                            = "internet-facing"
            "service.beta.kubernetes.io/aws-load-balancer-cross-zone-load-balancing-enabled" = "true"
          }
        }
        config = {
          "use-proxy-protocol"   = "false"
          "proxy-body-size"      = "10m"
          "client-max-body-size" = "10m"
        }
      }
      defaultBackend = {
        enabled = true
        image = {
          repository = "registry.k8s.io/defaultbackend-amd64"
          tag        = "1.5"
        }
        resources = {
          limits = {
            cpu    = "10m"
            memory = "20Mi"
          }
          requests = {
            cpu    = "10m"
            memory = "20Mi"
          }
        }
      }
    })
  ]

  lifecycle {
    create_before_destroy = true
  }
}

# Get the NLB DNS name once it's created
data "kubernetes_service" "nginx_ingress" {
  depends_on = [helm_release.nginx_ingress]

  metadata {
    name      = "nginx-ingress-controller-controller"
    namespace = "ingress-nginx"
  }
}

# Create Route53 record for tools.frontal.dev
resource "aws_route53_record" "tools_frontal_dev" {
  zone_id = data.aws_route53_zone.frontal_dev.zone_id
  name    = "tools.frontal.dev"
  type    = "A"

  alias {
    name                   = data.kubernetes_service.nginx_ingress.status.0.load_balancer.0.ingress.0.hostname
    zone_id                = data.kubernetes_service.nginx_ingress.status.0.load_balancer.0.ingress.0.zone_id
    evaluate_target_health = true
  }
}
