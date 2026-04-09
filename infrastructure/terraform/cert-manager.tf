# Install cert-manager for SSL certificates
resource "helm_release" "cert_manager" {
  name       = "cert-manager"
  repository = "https://charts.jetstack.io"
  chart      = "cert-manager"
  namespace  = "cert-manager"
  version    = "v1.13.0"

  create_namespace = true

  set {
    name  = "installCRDs"
    value = "true"
  }

  set {
    name  = "prometheus.enabled"
    value = "false"
  }

  values = [
    yamlencode({
      webhook = {
        timeoutSeconds = 30
      }
      cainjector = {
        replicaCount = 1
        resources = {
          limits = {
            cpu    = "100m"
            memory = "100Mi"
          }
          requests = {
            cpu    = "100m"
            memory = "100Mi"
          }
        }
      }
      controller = {
        replicaCount = 1
        resources = {
          limits = {
            cpu    = "100m"
            memory = "100Mi"
          }
          requests = {
            cpu    = "100m"
            memory = "100Mi"
          }
        }
      }
    })
  ]
}

# Create Let's Encrypt ClusterIssuer
resource "kubernetes_manifest" "letsencrypt_issuer" {
  manifest = {
    apiVersion = "cert-manager.io/v1"
    kind       = "ClusterIssuer"
    metadata = {
      name = "letsencrypt-prod"
    }
    spec = {
      acme = {
        server = "https://acme-v02.api.letsencrypt.org/directory"
        email  = "admin@frontal.dev"
        privateKeySecretRef = {
          name = "letsencrypt-prod-private-key"
        }
        solvers = [
          {
            http01 = {
              ingress = {
                class = "nginx"
              }
            }
          }
        ]
      }
    }
  }

  depends_on = [helm_release.cert_manager]
}
