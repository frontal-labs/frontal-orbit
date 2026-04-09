# Containers Guide

This guide covers containerization and deployment of Orbit CLI using Docker, Kubernetes, and other container technologies.

## Table of Contents

- [Overview](#overview)
- [Docker Images](#docker-images)
- [Docker Compose](#docker-compose)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Container Security](#container-security)
- [Performance Optimization](#performance-optimization)
- [Monitoring and Logging](#monitoring-and-logging)
- [CI/CD Integration](#cicd-integration)
- [Troubleshooting](#troubleshooting)

## Overview

Orbit CLI provides official container images for easy deployment and scaling. Containerization offers:

- **Consistency**: Same environment across development, staging, and production
- **Portability**: Run on any platform supporting containers
- **Scalability**: Easy horizontal scaling with orchestration
- **Isolation**: Security and dependency isolation
- **Versioning**: Immutable deployments with version control

### Container Benefits

- **Rapid Deployment**: Spin up new instances in seconds
- **Resource Efficiency**: Shared kernel and optimized resource usage
- **DevOps Integration**: Fits into modern CI/CD pipelines
- **Rollback Capability**: Easy rollback to previous versions
- **Environment Parity**: Eliminate "it works on my machine" issues

## Docker Images

### Official Images

Orbit provides official Docker images on multiple registries:

Pin an explicit release tag or digest in production. Avoid mutable aliases like `latest`.

#### Docker Hub

```bash
# Pinned release
docker pull orbit/cli:v0.1.0

# Specific version
docker pull orbit/cli:v0.1.0

# Alpine variant (smaller size)
docker pull orbit/cli:alpine

# Development version
docker pull orbit/cli:dev
```

#### GitHub Container Registry

```bash
# Pinned release
docker pull ghcr.io/orbit-org/cli:v0.1.0

# Specific version
docker pull ghcr.io/orbit-org/cli:v0.1.0
```

#### Amazon ECR

```bash
# Public ECR
docker pull public.ecr.aws/orbit/cli:v0.1.0
```

### Image Variants

| Variant | Description | Size | Use Case |
|---------|-------------|--------|----------|
| `vX.Y.Z` | Pinned release tag | General purpose |
| `alpine` | Alpine Linux-based | Minimal size, security-focused |
| `slim` | Slimmed-down image | Reduced attack surface |
| `dev` | Development build with tools | Development and debugging |

### Image Layers

```
orbit/cli:v0.1.0
├── rust:1.75-alpine          # Base runtime
├── ca-certificates             # SSL certificates
├── orbit-cli-binary           # Compiled Orbit binary
├── configuration-templates     # Default config files
├── plugins                   # Built-in plugins
└── entrypoint-scripts        # Startup and health scripts
```

### Building Custom Images

#### Dockerfile

```dockerfile
# Multi-stage build for smaller final image
FROM rust:1.75-alpine AS builder

# Install dependencies
RUN apk add --no-cache musl-dev

# Build Orbit
WORKDIR /app
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

# Final runtime image
FROM alpine:3.20

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    curl \
    bash

# Create non-root user
RUN addgroup -g 1000 orbit && \
    adduser -D -s /bin/sh -u 1000 -G orbit orbit

# Copy binary and set permissions
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/orbit /usr/local/bin/
RUN chmod +x /usr/local/bin/orbit

# Set up directories
RUN mkdir -p /home/orbit/.orbit && \
    chown -R orbit:orbit /home/orbit

# Switch to non-root user
USER orbit

# Set environment
ENV ORBIT_DATA_DIR=/home/orbit/.orbit
ENV PATH=/usr/local/bin:$PATH

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD orbit status || exit 1

# Entry point
ENTRYPOINT ["orbit"]
CMD ["--help"]
```

#### Build and Push

```bash
# Build image
docker build -t orbit/cli:custom .

# Tag for registry
docker tag orbit/cli:custom ghcr.io/orbit-org/cli:custom

# Push to registry
docker push ghcr.io/orbit-org/cli:custom
```

### Multi-Architecture Builds

```dockerfile
# Build for multiple architectures
FROM --platform=linux/amd64 rust:1.75-alpine AS builder-amd64
FROM --platform=linux/arm64 rust:1.75-alpine AS builder-arm64

# Build steps...

# Final image with multiple architectures
FROM --platform=linux/amd64 alpine:3.20
# ... copy from builder-amd64

# Create manifest
docker manifest create orbit/cli:multiarch \
    orbit/cli:amd64 \
    orbit/cli:arm64

docker manifest push orbit/cli:multiarch
```

## Docker Compose

### Development Environment

```yaml
# docker-compose.dev.yml
version: '3.8'

services:
  orbit:
    build:
      context: .
      dockerfile: Dockerfile.dev
    ports:
      - "8080:8080"
    volumes:
      - ./:/app
      - orbit-data:/home/orbit/.orbit
    environment:
      - ORBIT_LOG_LEVEL=debug
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - ORBIT_DEFAULT_MODEL=claude-sonnet-4-6
    working_dir: /app
    command: repl
    restart: unless-stopped

volumes:
  orbit-data:
    driver: local
```

Only mount `/var/run/docker.sock` when the container must launch sibling Docker workers.
Keep it off the default development path and add it explicitly for trusted local-docker scenarios.

### Production Environment

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  orbit:
    image: orbit/cli:v0.1.0
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '1.0'
          memory: 1G
        reservations:
          cpus: '0.5'
          memory: 512M
      restart_policy:
        condition: on-failure
        delay: 5s
        max_attempts: 3
    environment:
      - ORBIT_LOG_LEVEL=info
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - ORBIT_PERMISSION_MODE=safe-mode
    volumes:
      - orbit-config:/home/orbit/.orbit
      - orbit-sessions:/home/orbit/.orbit/sessions
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
    healthcheck:
      test: ["CMD", "orbit", "status"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  redis:
    image: redis:7-alpine
    deploy:
      resources:
        limits:
          cpus: '0.5'
          memory: 256M
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes

volumes:
  orbit-config:
    driver: local
  orbit-sessions:
    driver: local
  redis-data:
    driver: local
```

### Monitoring Stack

```yaml
# docker-compose.monitoring.yml
version: '3.8'

services:
  orbit:
    image: orbit/cli:v0.1.0
    environment:
      - ORBIT_TELEMETRY_ENABLED=true
      - ORBIT_TELEMETRY_ENDPOINT=http://prometheus:9090/metrics
    depends_on:
      - prometheus
      - grafana

  prometheus:
    image: prom/prometheus:vX.Y.Z
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'

  grafana:
    image: grafana/grafana:X.Y.Z
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards

volumes:
  prometheus-data:
  grafana-data:
```

## Kubernetes Deployment

### Namespace and RBAC

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: orbit
  labels:
    name: orbit

---
# rbac.yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: orbit-sa
  namespace: orbit

---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: orbit-role
  namespace: orbit
rules:
- apiGroups: [""]
  resources: ["pods", "services", "configmaps"]
  verbs: ["get", "list", "watch"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: orbit-rolebinding
  namespace: orbit
subjects:
- kind: ServiceAccount
  name: orbit-sa
  namespace: orbit
roleRef:
  kind: Role
  name: orbit-role
```

### Deployment Configuration

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: orbit-cli
  namespace: orbit
  labels:
    app: orbit-cli
    version: v1.0.0
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: orbit-cli
  template:
    metadata:
      labels:
        app: orbit-cli
        version: v1.0.0
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: orbit-sa
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        runAsGroup: 1000
        fsGroup: 1000
      containers:
      - name: orbit-cli
        image: orbit/cli:v0.1.0
        imagePullPolicy: Always
        ports:
        - name: http
          containerPort: 8080
          protocol: TCP
        env:
        - name: ORBIT_LOG_LEVEL
          value: "info"
        - name: ANTHROPIC_API_KEY
          valueFrom:
            secretKeyRef:
              name: orbit-secrets
              key: anthropic-api-key
        - name: ORBIT_DEFAULT_MODEL
          value: "claude-sonnet-4-6"
        - name: ORBIT_PERMISSION_MODE
          value: "safe-mode"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
        volumeMounts:
        - name: orbit-config
          mountPath: /home/orbit/.orbit
          readOnly: false
        - name: orbit-sessions
          mountPath: /home/orbit/.orbit/sessions
          readOnly: false
      volumes:
      - name: orbit-config
        persistentVolumeClaim:
          claimName: orbit-config-pvc
      - name: orbit-sessions
        persistentVolumeClaim:
          claimName: orbit-sessions-pvc
      restartPolicy: Always
      terminationGracePeriodSeconds: 30
```

### Service Configuration

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: orbit-service
  namespace: orbit
  labels:
    app: orbit-cli
spec:
  type: ClusterIP
  ports:
  - name: http
    port: 80
    targetPort: 8080
    protocol: TCP
  selector:
    app: orbit-cli

---
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: orbit-ingress
  namespace: orbit
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/rewrite-target: /
spec:
  tls:
  - hosts:
    - orbit.example.com
    secretName: orbit-tls
  rules:
  - host: orbit.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: orbit-service
            port:
              number: 80
```

### Persistent Storage

```yaml
# pvc.yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: orbit-config-pvc
  namespace: orbit
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 1Gi
  storageClassName: fast-ssd

---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: orbit-sessions-pvc
  namespace: orbit
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: fast-ssd
```

### Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: orbit-hpa
  namespace: orbit
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: orbit-cli
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 10
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
```

## Container Security

### Security Best Practices

#### Non-Root User

```dockerfile
# Create non-root user
RUN addgroup -g 1000 orbit && \
    adduser -D -s /bin/sh -u 1000 -G orbit orbit

# Use non-root user
USER orbit
```

#### Read-Only Filesystem

```dockerfile
# Copy as read-only
COPY --chown=orbit:orbit . /app
RUN chmod -R 755 /app

# Mount read-only where possible
VOLUME ["/home/orbit/.orbit:rw"]
```

#### Minimal Attack Surface

```dockerfile
# Use Alpine for minimal base
FROM alpine:3.20

# Install only required packages
RUN apk add --no-cache \
    ca-certificates \
    curl \
    && rm -rf /var/cache/apk/*

# Remove unnecessary tools
RUN rm -rf /usr/bin/wget \
    /usr/bin/curl \
    /bin/sh
```

### Security Context

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  runAsGroup: 1000
  fsGroup: 1000
  capabilities:
    drop:
    - ALL
  readOnlyRootFilesystem: false
  allowPrivilegeEscalation: false
```

### Pod Security Policies

```yaml
# pod-security-policy.yaml
apiVersion: policy/v1beta1
kind: PodSecurityPolicy
metadata:
  name: orbit-psp
spec:
  privileged: false
  allowPrivilegeEscalation: false
  requiredDropCapabilities:
    - ALL
  volumes:
    - 'configMap'
    - 'emptyDir'
    - 'projected'
    - 'secret'
    - 'downwardAPI'
    - 'persistentVolumeClaim'
  runAsUser:
    rule: 'MustRunAsNonRoot'
  seLinux:
    rule: 'RunAsAny'
  fsGroup:
    rule: 'RunAsAny'
```

### Secrets Management

```yaml
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: orbit-secrets
  namespace: orbit
type: Opaque
data:
  anthropic-api-key: <base64-encoded-key>
  openai-api-key: <base64-encoded-key>
  xai-api-key: <base64-encoded-key>

---
# sealed-secrets.yaml (using Sealed Secrets)
apiVersion: bitnami.com/v1alpha1
kind: SealedSecret
metadata:
  name: orbit-secrets
  namespace: orbit
spec:
  encryptedData:
    anthropic-api-key: <encrypted-key>
    openai-api-key: <encrypted-key>
    xai-api-key: <encrypted-key>
```

## Performance Optimization

### Resource Limits

```yaml
resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "1Gi"
    cpu: "1000m"
```

### Caching Strategy

```yaml
# Redis cache sidecar
- name: redis-cache
  image: redis:7-alpine
  resources:
    limits:
      memory: "256Mi"
      cpu: "250m"
  env:
    - name: REDIS_MAXMEMORY
      value: "200mb"
```

### Connection Pooling

```yaml
env:
  - name: ORBIT_CONNECTION_POOL_SIZE
    value: "10"
  - name: ORBIT_CONNECTION_TIMEOUT
    value: "30"
  - name: ORBIT_KEEP_ALIVE
    value: "true"
```

### Image Optimization

```dockerfile
# Multi-stage build for smaller images
FROM rust:1.75-alpine AS builder
# ... build steps ...

FROM scratch
COPY --from=builder /app/target/release/orbit /orbit
# No additional layers for minimal size
```

## Monitoring and Logging

### Health Checks

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD orbit status --json || exit 1
```

### Metrics Collection

```yaml
# prometheus-config.yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'orbit'
    static_configs:
      - targets: ['orbit:8080']
    metrics_path: /metrics
    scrape_interval: 5s
```

### Log Aggregation

```yaml
# fluentd-config.yaml
<source>
  @type tail
  path /var/log/containers/*.log
  pos_file /var/log/fluentd-containers.log.pos
  tag kubernetes.*
  read_from_head true
  <parse>
    @type json
    time_format %Y-%m-%dT%H:%M:%S.%NZ
  </parse>
</source>

<match kubernetes.**>
  @type elasticsearch
  host elasticsearch
  port 9200
  index_name orbit-logs
  type_name _doc
</match>
```

### Distributed Tracing

```yaml
# jaeger-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: jaeger
spec:
  template:
    spec:
      containers:
      - name: jaeger
        image: jaegertracing/all-in-one:X.Y.Z
        ports:
        - containerPort: 16686
          name: ui
        - containerPort: 14268
          name: collector
        env:
        - name: COLLECTOR_ZIPKIN_HTTP_PORT
          value: "9411"
        - name: SPAN_STORAGE_TYPE
          value: "elasticsearch"
        - name: ES_SERVER_URLS
          value: "http://elasticsearch:9200"
```

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/docker.yml
name: Build and Deploy Docker

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-24.04
    steps:
    - uses: actions/checkout@v3
    - name: Set up Docker Buildx
      uses: docker/setup-buildx-action@v2
    - name: Build Docker image
      run: |
        docker buildx build \
          --platform linux/amd64,linux/arm64 \
          --tag orbit/cli:${{ github.sha }} \
          --load .
    - name: Test Docker image
      run: |
        docker run --rm orbit/cli:${{ github.sha }} --version

  build-and-push:
    needs: test
    runs-on: ubuntu-24.04
    if: github.ref == 'refs/heads/main'
    steps:
    - uses: actions/checkout@v3
    - name: Set up Docker Buildx
      uses: docker/setup-buildx-action@v2
    - name: Login to Container Registry
      uses: docker/login-action@v2
      with:
        registry: ghcr.io
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}
    - name: Build and push Docker image
      run: |
        docker buildx build \
          --platform linux/amd64,linux/arm64 \
          --tag ghcr.io/orbit-org/cli:${{ github.sha }} \
          --push .
```

### GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - test
  - build
  - deploy

variables:
  DOCKER_DRIVER: overlay2
  DOCKER_TLS_CERTDIR: "/certs"

services:
  - docker:dind

test:
  stage: test
  script:
    - docker build -t orbit/cli:test .
    - docker run --rm orbit/cli:test --version

build:
  stage: build
  script:
    - docker build -t $CI_REGISTRY_IMAGE:$CI_COMMIT_SHA .
    - docker push $CI_REGISTRY_IMAGE:$CI_COMMIT_SHA
  only:
    - main

deploy:
  stage: deploy
  script:
    - kubectl set image deployment/orbit-cli orbit=$CI_REGISTRY_IMAGE:$CI_COMMIT_SHA
    - kubectl rollout status deployment/orbit-cli
  only:
    - main
```

### ArgoCD

```yaml
# argocd-application.yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: orbit-cli
  namespace: argocd
spec:
  project: default
  source:
    repoURL: https://github.com/orbit-org/orbit-k8s.git
    targetRevision: HEAD
    path: manifests
  destination:
    server: https://kubernetes.default.svc
    namespace: orbit
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
    - CreateNamespace=true
  retry:
    limit: 5
    backoff:
      duration: 5s
      factor: 2
      maxDuration: 3m
```

## Troubleshooting

### Common Issues

#### Container Won't Start

```bash
# Check logs
docker logs orbit-container

# Check health status
docker inspect orbit-container --format='{{.State.Health.Status}}'

# Debug with interactive shell
docker run -it --entrypoint /bin/sh orbit/cli:v0.1.0
```

#### Permission Issues

```bash
# Check user permissions
docker run orbit/cli:v0.1.0 id

# Fix volume permissions
docker run --user 1000:1000 -v $(pwd):/app orbit/cli:v0.1.0

# Use security context
docker run --security-opt no-new-privileges orbit/cli:v0.1.0
```

#### Resource Issues

```bash
# Monitor resource usage
docker stats orbit-container

# Check limits
docker inspect orbit-container --format='{{.HostConfig.Resources}}'

# Adjust limits
docker update --memory=2g --cpus=2 orbit-container
```

### Debugging Tools

```bash
# Enter running container
docker exec -it orbit-container /bin/sh

# Monitor network traffic
docker run --network container:orbit-container nicolaka/netshoot

# Check filesystem
docker run --volumes-from orbit-container busybox ls -la /home/orbit/.orbit
```

### Performance Debugging

```bash
# Profile with perf
docker run --privileged -v /usr/local/bin/perf:/usr/local/bin/perf orbit/cli:v0.1.0

# Memory profiling
docker run --memory=512m --memory-swap=512m orbit/cli:v0.1.0

# CPU profiling
docker run --cpus=0.5 orbit/cli:v0.1.0
```

This containers guide provides comprehensive coverage of deploying and managing Orbit CLI in containerized environments.
