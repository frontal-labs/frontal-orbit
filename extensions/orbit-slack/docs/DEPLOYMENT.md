# Orbit Slack Extension Deployment Guide

## Overview

This guide covers deployment options and best practices for the Orbit Slack extension in production environments.

## Deployment Options

### 1. Docker Deployment (Recommended)

Docker provides a consistent, isolated environment for running the extension.

#### Building the Image

```bash
# Build the Docker image
docker build -t orbit-slack:latest .

# Build with specific tag
docker build -t orbit-slack:v0.1.0 .
```

#### Running the Container

```bash
# Basic deployment
docker run -d \
  --name orbit-slack \
  -p 3000:3000 \
  --env-file .env \
  orbit-slack:latest

# With volume for logs
docker run -d \
  --name orbit-slack \
  -p 3000:3000 \
  --env-file .env \
  -v /path/to/logs:/app/logs \
  orbit-slack:latest
```

#### Docker Compose

```yaml
version: '3.8'

services:
  orbit-slack:
    build: .
    ports:
      - "3000:3000"
    env_file:
      - .env
    volumes:
      - ./logs:/app/logs
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

### 2. Kubernetes Deployment

For scalable, production deployments.

#### Deployment Manifest

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: orbit-slack
  labels:
    app: orbit-slack
spec:
  replicas: 2
  selector:
    matchLabels:
      app: orbit-slack
  template:
    metadata:
      labels:
        app: orbit-slack
    spec:
      containers:
      - name: orbit-slack
        image: orbit-slack:latest
        ports:
        - containerPort: 3000
        env:
        - name: NODE_ENV
          value: "production"
        - name: SLACK_BOT_TOKEN
          valueFrom:
            secretKeyRef:
              name: slack-secrets
              key: bot-token
        - name: SLACK_APP_TOKEN
          valueFrom:
            secretKeyRef:
              name: slack-secrets
              key: app-token
        - name: SLACK_SIGNING_SECRET
          valueFrom:
            secretKeyRef:
              name: slack-secrets
              key: signing-secret
        - name: ORBIT_API_URL
          value: "http://orbit-api:8788"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: orbit-slack-service
spec:
  selector:
    app: orbit-slack
  ports:
  - protocol: TCP
    port: 80
    targetPort: 3000
  type: ClusterIP
---
apiVersion: v1
kind: Secret
metadata:
  name: slack-secrets
type: Opaque
data:
  bot-token: <base64-encoded-token>
  app-token: <base64-encoded-token>
  signing-secret: <base64-encoded-secret>
```

### 3. Direct Node.js Deployment

For simple deployments without containerization.

#### Prerequisites

```bash
# Install Node.js 18+
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Install Bun globally
npm install -g bun

# Create system user
sudo useradd -r -s /bin/false orbit
```

#### Deployment Steps

```bash
# Clone and build
git clone <repository-url> /opt/orbit-slack
cd /opt/orbit-slack
bun install
bun run build

# Create systemd service
sudo tee /etc/systemd/system/orbit-slack.service > /dev/null <<EOF
[Unit]
Description=Orbit Slack Extension
After=network.target

[Service]
Type=simple
User=orbit
WorkingDirectory=/opt/orbit-slack
Environment=NODE_ENV=production
EnvironmentFile=/opt/orbit-slack/.env
ExecStart=/usr/bin/bun start
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl enable orbit-slack
sudo systemctl start orbit-slack
```

## Environment Configuration

### Production Environment Variables

```bash
# Slack Configuration
SLACK_BOT_TOKEN=xoxb-production-bot-token
SLACK_APP_TOKEN=xapp-production-app-token
SLACK_SIGNING_SECRET=production-signing-secret

# Orbit Server
ORBIT_API_URL=https://orbit-api.your-domain.com
ORBIT_API_TIMEOUT=30000

# Application
NODE_ENV=production
LOG_LEVEL=info
PORT=3000

# Optional: Monitoring
SENTRY_DSN=https://your-sentry-dsn

# Performance Tuning
MAX_CONCURRENT_TASKS=20
TASK_TIMEOUT=7200000
HEALTH_CHECK_INTERVAL=30000
```

### Environment File Management

#### Docker Secrets (Recommended)

```bash
# Create secrets
echo "xoxb-your-bot-token" | docker secret create slack-bot-token -
echo "xapp-your-app-token" | docker secret create slack-app-token -
echo "your-signing-secret" | docker secret create slack-signing-secret -
```

#### Kubernetes Secrets

```bash
# Create secrets from files
kubectl create secret generic slack-secrets \
  --from-file=bot-token=slack-bot-token.txt \
  --from-file=app-token=slack-app-token.txt \
  --from-file=signing-secret=slack-signing-secret.txt
```

## Scaling Considerations

### Horizontal Scaling

The extension can be scaled horizontally with these considerations:

1. **Stateless Design**: The extension is designed to be stateless
2. **WebSocket Connections**: Each instance manages its own WebSocket connections
3. **Task Distribution**: Tasks are distributed across instances via Orbit server

#### Load Balancing

```yaml
# Kubernetes Horizontal Pod Autoscaler
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: orbit-slack-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: orbit-slack
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
```

### Resource Requirements

#### Minimum Requirements

- **CPU**: 250m per instance
- **Memory**: 256Mi per instance
- **Network**: 100Mbps
- **Storage**: 1Gi (for logs)

#### Recommended Production

- **CPU**: 500m per instance
- **Memory**: 512Mi per instance
- **Network**: 1Gbps
- **Storage**: 5Gi (for logs and metrics)

## Monitoring and Observability

### Health Checks

The extension provides comprehensive health checks:

```bash
# Basic health check
curl http://localhost:3000/health

# Detailed health check
curl http://localhost:3000/health?detailed=true
```

#### Health Check Response

```json
{
  "status": "ok",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "0.1.0",
  "uptime": 86400,
  "checks": {
    "orbit_api": "ok",
    "slack_connection": "ok",
    "websocket": "ok",
    "memory": "ok"
  }
}
```

### Logging

#### Structured Logging

```typescript
// Example log entry
{
  "level": "info",
  "timestamp": "2024-01-01T00:00:00Z",
  "service": "orbit-slack",
  "message": "Task created successfully",
  "taskId": "task_123",
  "slackUserId": "U1234567890",
  "duration": 1250
}
```

#### Log Aggregation

```yaml
# Fluentd configuration for log aggregation
<source>
  @type tail
  path /var/log/orbit-slack/*.log
  pos_file /var/log/fluentd/orbit-slack.log.pos
  tag orbit-slack.*
  format json
</source>

<match orbit-slack.**>
  @type elasticsearch
  host elasticsearch
  port 9200
  index_name orbit-slack
  type_name _doc
</match>
```

### Metrics

#### Prometheus Metrics

```typescript
// Custom metrics
const taskCreationCounter = new Counter({
  name: 'orbit_slack_tasks_created_total',
  help: 'Total number of tasks created'
});

const taskDurationHistogram = new Histogram({
  name: 'orbit_slack_task_duration_seconds',
  help: 'Task execution duration'
});
```

#### Grafana Dashboard

Create a Grafana dashboard with these panels:

1. **Task Creation Rate**: Tasks created per minute
2. **Task Completion Rate**: Tasks completed per minute
3. **Error Rate**: Error percentage by type
4. **Response Time**: API response times
5. **WebSocket Connections**: Active WebSocket connections
6. **Memory Usage**: Memory consumption over time

## Security

### Network Security

#### Firewall Rules

```bash
# Allow inbound traffic to port 3000
sudo ufw allow 3000/tcp

# Allow outbound traffic to Orbit API
sudo ufw allow out to <orbit-api-ip> port 8788

# Allow outbound traffic to Slack API
sudo ufw allow out to api.slack.com port 443
```

#### SSL/TLS

```nginx
# Nginx reverse proxy with SSL
server {
    listen 443 ssl http2;
    server_name slack.your-domain.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Secret Management

#### HashiCorp Vault

```bash
# Store secrets in Vault
vault kv put secret/orbit-slack/slack \
  bot_token=xoxb-your-bot-token \
  app_token=xapp-your-app-token \
  signing_secret=your-signing-secret

# Retrieve secrets in application
vault kv get -field=bot_token secret/orbit-slack/slack
```

#### AWS Secrets Manager

```typescript
import AWS from 'aws-sdk';

const secretsManager = new AWS.SecretsManager();

async function getSecret(secretName: string): Promise<string> {
  const data = await secretsManager.getSecretValue({ SecretId: secretName }).promise();
  return data.SecretString;
}
```

## Backup and Recovery

### Data Backup

The extension doesn't store persistent data, but you should backup:

1. **Configuration files**: `.env`, `bunfig.toml`
2. **Logs**: For audit trails
3. **Docker images**: Version control

#### Backup Script

```bash
#!/bin/bash
# backup-orbit-slack.sh

BACKUP_DIR="/backup/orbit-slack/$(date +%Y%m%d)"
mkdir -p "$BACKUP_DIR"

# Backup configuration
cp .env "$BACKUP_DIR/"
cp bunfig.toml "$BACKUP_DIR/"

# Backup logs
cp -r logs/ "$BACKUP_DIR/"

# Backup Docker image
docker save orbit-slack:latest | gzip > "$BACKUP_DIR/orbit-slack.tar.gz"

echo "Backup completed: $BACKUP_DIR"
```

### Disaster Recovery

#### Recovery Steps

1. **Restore Configuration**: Copy `.env` and `bunfig.toml`
2. **Deploy Application**: Use Docker or systemd deployment
3. **Verify Health**: Check health endpoint
4. **Test Integration**: Verify Slack and Orbit connectivity

#### Recovery Time Objective (RTO)

- **Quick Recovery**: 5 minutes (restart existing deployment)
- **Full Recovery**: 30 minutes (redeploy from backup)

## Performance Optimization

### Caching

#### Redis Cache

```typescript
import Redis from 'ioredis';

const redis = new Redis(process.env.REDIS_URL);

// Cache task data
await redis.setex(`task:${taskId}`, 300, JSON.stringify(task));

// Retrieve cached task
const cachedTask = await redis.get(`task:${taskId}`);
```

### Connection Pooling

```typescript
// HTTP client with connection pooling
import { Agent } from 'undici';

const agent = new Agent({
  connections: 10,
  keepAliveTimeout: 60000
});
```

### Memory Optimization

```typescript
// Set garbage collection options
process.env.NODE_OPTIONS = '--max-old-space-size=512';

// Monitor memory usage
setInterval(() => {
  const usage = process.memoryUsage();
  console.log('Memory usage:', usage);
}, 60000);
```

## Troubleshooting

### Common Deployment Issues

#### 1. WebSocket Connection Failures

**Symptoms**: Tasks not updating in Slack

**Solutions**:
- Check firewall rules
- Verify Orbit API URL
- Review WebSocket logs
- Test connectivity: `telnet orbit-api 8788`

#### 2. High Memory Usage

**Symptoms**: Container OOM kills

**Solutions**:
- Increase memory limits
- Profile memory usage
- Check for memory leaks
- Optimize garbage collection

#### 3. Slack Rate Limiting

**Symptoms**: Messages not posting

**Solutions**:
- Implement rate limiting
- Use queue for message posting
- Monitor Slack API limits
- Batch API calls

### Debug Mode

Enable debug logging for troubleshooting:

```bash
LOG_LEVEL=debug bun run dev
```

### Log Analysis

```bash
# Filter error logs
grep "ERROR" logs/app.log

# Analyze task creation patterns
grep "Task created" logs/app.log | awk '{print $1}' | sort | uniq -c

# Monitor WebSocket connections
grep "WebSocket" logs/app.log | tail -f
```

## Maintenance

### Regular Maintenance Tasks

1. **Weekly**: Review logs and metrics
2. **Monthly**: Update dependencies
3. **Quarterly**: Security audit
4. **Annually**: Architecture review

### Dependency Updates

```bash
# Update dependencies
bun update

# Check for security vulnerabilities
bun audit

# Update Docker base image
FROM node:18-alpine@sha256:<new-sha>
```

### Security Patching

```bash
# Check for security updates
bun audit

# Apply security patches
bun update --latest

# Rebuild and redeploy
docker build -t orbit-slack:patched .
docker run -d --name orbit-slack-patched orbit-slack:patched
```
