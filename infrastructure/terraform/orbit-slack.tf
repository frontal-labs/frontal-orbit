# Deploy orbit-slack if enabled
resource "kubernetes_config_map" "orbit_slack_config" {
  count = var.deploy_orbit_slack ? 1 : 0
  metadata {
    name      = "orbit-slack-config"
    namespace = var.orbit_service_namespace
    labels = {
      app         = "orbit-slack"
      environment = var.environment
    }
  }

  data = {
    "ORBIT_API_URL"         = "http://orbit-server:8788"
    "ORBIT_API_TIMEOUT"     = "30000"
    "NODE_ENV"              = var.environment
    "LOG_LEVEL"             = "info"
    "PORT"                  = "3000"
    "MAX_CONCURRENT_TASKS"  = "10"
    "TASK_TIMEOUT"          = "3600000"
    "HEALTH_CHECK_INTERVAL" = "30000"
  }

  depends_on = [kubernetes_namespace.orbit]
}

# Secrets for orbit-slack
resource "kubernetes_secret" "orbit_slack_secrets" {
  count = var.deploy_orbit_slack ? 1 : 0
  metadata {
    name      = "orbit-slack-secrets"
    namespace = var.orbit_service_namespace
    labels = {
      app         = "orbit-slack"
      environment = var.environment
    }
  }

  data = {
    "ORBIT_API_KEY"        = var.orbit_server_api_key
    "SLACK_BOT_TOKEN"      = var.slack_bot_token
    "SLACK_APP_TOKEN"      = var.slack_app_token
    "SLACK_SIGNING_SECRET" = var.slack_signing_secret
    "GITHUB_TOKEN"         = var.github_token
    "SENTRY_DSN"           = var.sentry_dsn
  }

  type = "Opaque"

  depends_on = [kubernetes_namespace.orbit]
}

# Deployment for orbit-slack
resource "kubernetes_deployment" "orbit_slack" {
  count = var.deploy_orbit_slack ? 1 : 0
  metadata {
    name      = "orbit-slack"
    namespace = var.orbit_service_namespace
    labels = {
      app         = "orbit-slack"
      environment = var.environment
    }
  }

  spec {
    replicas = 2

    selector {
      match_labels = {
        app = "orbit-slack"
      }
    }

    template {
      metadata {
        labels = {
          app         = "orbit-slack"
          environment = var.environment
        }
      }

      spec {
        automount_service_account_token = false

        container {
          name              = "orbit-slack"
          image             = var.orbit_slack_image
          image_pull_policy = "IfNotPresent"

          security_context {
            allow_privilege_escalation = false
            read_only_root_filesystem  = true
            run_as_group               = 1001
            run_as_non_root            = true
            run_as_user                = 1001
          }

          port {
            container_port = 3000
            name           = "http"
          }

          env_from {
            config_map_ref {
              name = "orbit-slack-config"
            }
          }

          env_from {
            secret_ref {
              name = "orbit-slack-secrets"
            }
          }

          volume_mount {
            name       = "tmp"
            mount_path = "/tmp"
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

          # Note: orbit-slack doesn't have HTTP health endpoint, using process check
          liveness_probe {
            exec {
              command = ["/bin/sh", "-c", "pidof node >/dev/null"]
            }
            initial_delay_seconds = 30
            period_seconds        = 10
            timeout_seconds       = 5
            failure_threshold     = 3
          }

          readiness_probe {
            exec {
              command = ["/bin/sh", "-c", "pidof node >/dev/null"]
            }
            initial_delay_seconds = 10
            period_seconds        = 5
            timeout_seconds       = 3
            failure_threshold     = 3
          }
        }

        volume {
          name = "tmp"
          empty_dir {}
        }

        affinity {
          pod_anti_affinity {
            preferred_during_scheduling_ignored_during_execution {
              weight = 100
              pod_affinity_term {
                label_selector {
                  match_labels = {
                    app = "orbit-slack"
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

  depends_on = [
    kubernetes_namespace.orbit,
    kubernetes_config_map.orbit_slack_config,
    kubernetes_secret.orbit_slack_secrets,
    kubernetes_deployment.orbit_server
  ]
}

# Service for orbit-slack
resource "kubernetes_service" "orbit_slack" {
  count = var.deploy_orbit_slack ? 1 : 0
  metadata {
    name      = "orbit-slack"
    namespace = var.orbit_service_namespace
    labels = {
      app         = "orbit-slack"
      environment = var.environment
    }
  }

  spec {
    selector = {
      app = "orbit-slack"
    }

    port {
      name        = "http"
      port        = 3000
      target_port = 3000
    }

    type = "ClusterIP"
  }

  depends_on = [kubernetes_deployment.orbit_slack]
}
