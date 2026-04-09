# Create Ingress for tools.frontal.dev/orbit
resource "kubernetes_ingress_v1" "orbit_tools" {
  metadata {
    name      = "orbit-tools-ingress"
    namespace = var.orbit_service_namespace
    annotations = {
      "kubernetes.io/ingress.class"                    = "nginx"
      "cert-manager.io/cluster-issuer"                 = "letsencrypt-prod"
      "nginx.ingress.kubernetes.io/ssl-redirect"       = "true"
      "nginx.ingress.kubernetes.io/use-regex"           = "true"
      "nginx.ingress.kubernetes.io/rewrite-target"      = "/$2"
      "nginx.ingress.kubernetes.io/proxy-body-size"     = "10m"
      "nginx.ingress.kubernetes.io/proxy-read-timeout"  = "300"
      "nginx.ingress.kubernetes.io/proxy-send-timeout"  = "300"
      "nginx.ingress.kubernetes.io/configuration-snippet" = <<-EOT
        more_set_headers "X-Forwarded-Proto: https";
        more_set_headers "X-Forwarded-Host: tools.frontal.dev";
      EOT
    }
  }

  spec {
    tls {
      hosts       = ["tools.frontal.dev"]
      secret_name = "tools-frontal-dev-tls"
    }

    rule {
      host = "tools.frontal.dev"
      http {
        path {
          path     = "/orbit(/|$)(.*)"
          pathType = "Prefix"
          backend {
            service {
              name = "orbit-server"
              port {
                number = 8788
              }
            }
          }
        }
      }
    }
  }

  depends_on = [helm_release.nginx_ingress]
}

# Create SSL certificate using cert-manager
resource "kubernetes_manifest" "orbit_tools_certificate" {
  manifest = {
    apiVersion = "cert-manager.io/v1"
    kind       = "Certificate"
    metadata = {
      name      = "tools-frontal-dev"
      namespace = var.orbit_service_namespace
    }
    spec = {
      secretName = "tools-frontal-dev-tls"
      dnsNames   = ["tools.frontal.dev"]
      issuerRef = {
        name  = "letsencrypt-prod"
        kind  = "ClusterIssuer"
      }
    }
  }

  depends_on = [kubernetes_ingress_v1.orbit_tools]
}
