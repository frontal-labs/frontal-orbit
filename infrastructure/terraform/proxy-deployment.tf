# Deploy the NGINX proxy as a Kubernetes deployment
resource "kubernetes_deployment" "tools_proxy" {
  metadata {
    name      = "tools-proxy"
    namespace = var.orbit_service_namespace
    labels = {
      app = "tools-proxy"
    }
  }

  spec {
    replicas = 2

    selector {
      match_labels = {
        app = "tools-proxy"
      }
    }

    template {
      metadata {
        labels = {
          app = "tools-proxy"
        }
      }

      spec {
        automount_service_account_token = false

        container {
          name  = "nginx"
          image = "nginx:1.27-alpine@sha256:65645c7bb6a0661892a8b03b89d0743208a18dd2f3f17a54ef4b76fb8e2f2a10"

          port {
            container_port = 80
            name           = "http"
          }

          port {
            container_port = 443
            name           = "https"
          }

          resources {
            limits = {
              cpu    = "500m"
              memory = "512Mi"
            }
            requests = {
              cpu    = "250m"
              memory = "256Mi"
            }
          }

          liveness_probe {
            http_get {
              path = "/health"
              port = 80
            }
            initial_delay_seconds = 30
            period_seconds        = 10
            timeout_seconds       = 5
            failure_threshold     = 3
          }

          readiness_probe {
            http_get {
              path = "/health"
              port = 80
            }
            initial_delay_seconds = 5
            period_seconds        = 5
            timeout_seconds       = 3
            failure_threshold     = 3
          }

          volume_mount {
            name       = "nginx-config"
            mount_path = "/etc/nginx/conf.d/tools.conf"
            sub_path   = "tools.conf"
          }

          volume_mount {
            name       = "ssl-certs"
            mount_path = "/etc/ssl/certs"
            read_only  = true
          }

          volume_mount {
            name       = "ssl-keys"
            mount_path = "/etc/ssl/private"
            read_only  = true
          }
        }

        volume {
          name = "nginx-config"
          config_map {
            name = "tools-proxy-config"
          }
        }

        volume {
          name = "ssl-certs"
          secret {
            secret_name = "tools-frontal-dev-tls"
          }
        }

        volume {
          name = "ssl-keys"
          secret {
            secret_name = "tools-frontal-dev-tls"
          }
        }

        affinity {
          pod_anti_affinity {
            preferred_during_scheduling_ignored_during_execution {
              weight = 100
              pod_affinity_term {
                label_selector {
                  match_labels = {
                    app = "tools-proxy"
                  }
                }
                topology_key = "kubernetes.io/hostname"
              }
            }
          }
        }
      }
    }
  }
}

# Create ConfigMap for nginx configuration
resource "kubernetes_config_map" "tools_proxy_config" {
  metadata {
    name      = "tools-proxy-config"
    namespace = var.orbit_service_namespace
  }

  data = {
    "tools.conf" = file("${path.module}/../proxy/nginx.conf")
  }
}

# Create Service for the proxy
resource "kubernetes_service" "tools_proxy" {
  metadata {
    name      = "tools-proxy"
    namespace = var.orbit_service_namespace
    labels = {
      app = "tools-proxy"
    }
  }

  spec {
    selector = {
      app = "tools-proxy"
    }

    port {
      name        = "http"
      port        = 80
      target_port = 80
    }

    port {
      name        = "https"
      port        = 443
      target_port = 443
    }

    type = "ClusterIP"
  }
}
