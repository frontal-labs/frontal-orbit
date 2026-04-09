# Deploy orbit-server if enabled
resource "kubernetes_namespace" "orbit" {
  count = var.deploy_orbit_server || var.deploy_orbit_slack ? 1 : 0
  metadata {
    name = var.orbit_service_namespace
    labels = {
      name = var.orbit_service_namespace
      environment = var.environment
    }
  }
}

# Persistent Volume Claims for orbit-server
resource "kubernetes_persistent_volume_claim" "orbit_workspace" {
  count = var.deploy_orbit_server ? 1 : 0
  metadata {
    name = "orbit-workspace"
    namespace = var.orbit_service_namespace
    labels = {
      app = "orbit-server"
      environment = var.environment
    }
  }

  spec {
    access_modes = ["ReadWriteOnce"]
    storage_class_name = var.storage_class
    resources {
      requests = {
        storage = var.workspace_storage_size
      }
    }
  }

  depends_on = [kubernetes_namespace.orbit]
}

resource "kubernetes_persistent_volume_claim" "orbit_server_state" {
  count = var.deploy_orbit_server ? 1 : 0
  metadata {
    name = "orbit-server-state"
    namespace = var.orbit_service_namespace
    labels = {
      app = "orbit-server"
      environment = var.environment
    }
  }

  spec {
    access_modes = ["ReadWriteOnce"]
    storage_class_name = var.storage_class
    resources {
      requests = {
        storage = var.server_state_storage_size
      }
    }
  }

  depends_on = [kubernetes_namespace.orbit]
}

resource "kubernetes_persistent_volume_claim" "orbit_agent_store" {
  count = var.deploy_orbit_server ? 1 : 0
  metadata {
    name = "orbit-agent-store"
    namespace = var.orbit_service_namespace
    labels = {
      app = "orbit-server"
      environment = var.environment
    }
  }

  spec {
    access_modes = ["ReadWriteOnce"]
    storage_class_name = var.storage_class
    resources {
      requests = {
        storage = var.agent_store_storage_size
      }
    }
  }

  depends_on = [kubernetes_namespace.orbit]
}

# ConfigMap for orbit-server configuration
resource "kubernetes_config_map" "orbit_server_config" {
  count = var.deploy_orbit_server ? 1 : 0
  metadata {
    name = "orbit-server-config"
    namespace = var.orbit_service_namespace
    labels = {
      app = "orbit-server"
      environment = var.environment
    }
  }

  data = {
    "ORBIT_SERVER_HOST" = "0.0.0.0"
    "ORBIT_SERVER_PORT" = "8788"
    "ORBIT_SERVER_LANE_TRANSPORT" = "tools-agent"
    "ORBIT_SERVER_RECONCILE_INTERVAL_SECS" = "15"
    "ORBIT_SERVER_ORPHAN_APPROVAL_DELAY_SECS" = "0"
    "ORBIT_SERVER_ORPHAN_AUTO_RETRY_SECS" = "0"
    "ORBIT_SERVER_ORPHAN_AUTO_CANCEL_SECS" = "0"
    "ORBIT_SERVER_ORPHAN_POLICY_RULES" = "[]"
    "ORBIT_SERVER_STATE_FILE" = "/var/lib/orbit/server/state.json"
    "ORBIT_AGENT_STORE" = "/var/lib/orbit/agents"
    "RUST_LOG" = "info"
  }

  depends_on = [kubernetes_namespace.orbit]
}

# Secrets for orbit-server
resource "kubernetes_secret" "orbit_server_secrets" {
  count = var.deploy_orbit_server ? 1 : 0
  metadata {
    name = "orbit-server-secrets"
    namespace = var.orbit_service_namespace
    labels = {
      app = "orbit-server"
      environment = var.environment
    }
  }

  data = {
    "ANTHROPIC_API_KEY" = var.api_keys.anthropic
    "OPENAI_API_KEY" = var.api_keys.openai
    "OPENAI_BASE_URL" = ""
    "FRONTAL_API_KEY" = var.api_keys.anthropic  # Use Anthropic key for Frontal
    "FRONTAL_BASE_URL" = "https://tools.frontal.dev/orbit"
    "XAI_API_KEY" = var.api_keys.xai
    "XAI_BASE_URL" = ""
    "AZURE_OPENAI_API_KEY" = var.api_keys.azure
    "AZURE_OPENAI_BASE_URL" = ""
    "BEDROCK_API_KEY" = var.api_keys.bedrock
    "BEDROCK_BASE_URL" = ""
    "OLLAMA_BASE_URL" = var.api_keys.ollama
    "OLLAMA_MODEL" = ""
  }

  type = "Opaque"

  depends_on = [kubernetes_namespace.orbit]
}

# Deployment for orbit-server
resource "kubernetes_deployment" "orbit_server" {
  count = var.deploy_orbit_server ? 1 : 0
  metadata {
    name = "orbit-server"
    namespace = var.orbit_service_namespace
    labels = {
      app = "orbit-server"
      environment = var.environment
    }
  }

  spec {
    replicas = 2

    selector {
      match_labels = {
        app = "orbit-server"
      }
    }

    template {
      metadata {
        labels = {
          app = "orbit-server"
          environment = var.environment
        }
      }

      spec {
        container {
          name = "orbit-server"
          image = var.orbit_server_image
          image_pull_policy = "IfNotPresent"

          port {
            container_port = 8788
            name = "http"
          }

          env_from {
            config_map_ref {
              name = "orbit-server-config"
            }
          }

          env_from {
            secret_ref {
              name = "orbit-server-secrets"
            }
          }

          resources {
            limits = {
              cpu = "2000m"
              memory = "4Gi"
            }
            requests = {
              cpu = "1000m"
              memory = "2Gi"
            }
          }

          liveness_probe {
            http_get {
              path = "/health"
              port = 8788
            }
            initial_delay_seconds = 30
            period_seconds = 10
            timeout_seconds = 5
            failure_threshold = 3
          }

          readiness_probe {
            http_get {
              path = "/health"
              port = 8788
            }
            initial_delay_seconds = 10
            period_seconds = 5
            timeout_seconds = 3
            failure_threshold = 3
          }

          volume_mount {
            name = "orbit-workspace"
            mount_path = "/workspace"
          }

          volume_mount {
            name = "orbit-server-state"
            mount_path = "/var/lib/orbit/server"
          }

          volume_mount {
            name = "orbit-agent-store"
            mount_path = "/var/lib/orbit/agents"
          }
        }

        volume {
          name = "orbit-workspace"
          persistent_volume_claim {
            claim_name = "orbit-workspace"
          }
        }

        volume {
          name = "orbit-server-state"
          persistent_volume_claim {
            claim_name = "orbit-server-state"
          }
        }

        volume {
          name = "orbit-agent-store"
          persistent_volume_claim {
            claim_name = "orbit-agent-store"
          }
        }

        affinity {
          pod_anti_affinity {
            preferred_during_scheduling_ignored_during_execution {
              weight = 100
              pod_affinity_term {
                label_selector {
                  match_labels = {
                    app = "orbit-server"
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
    kubernetes_config_map.orbit_server_config,
    kubernetes_secret.orbit_server_secrets,
    kubernetes_persistent_volume_claim.orbit_workspace,
    kubernetes_persistent_volume_claim.orbit_server_state,
    kubernetes_persistent_volume_claim.orbit_agent_store
  ]
}

# Service for orbit-server
resource "kubernetes_service" "orbit_server" {
  count = var.deploy_orbit_server ? 1 : 0
  metadata {
    name = "orbit-server"
    namespace = var.orbit_service_namespace
    labels = {
      app = "orbit-server"
      environment = var.environment
    }
  }

  spec {
    selector = {
      app = "orbit-server"
    }

    port {
      name = "http"
      port = 8788
      target_port = 8788
    }

    type = "ClusterIP"
  }

  depends_on = [kubernetes_deployment.orbit_server]
}
