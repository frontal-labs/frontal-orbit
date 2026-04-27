# Docker Setup for Frontal Orbit

This guide covers how to use Docker with Frontal Orbit for development, testing, and production deployments.

## Quick Start

### 1. Environment Setup

```bash
# Copy environment template
cp .env.example .env

# Edit with your API keys
nano .env
```

### 2. Development Environment

```bash
# Start development CLI with hot reload
docker-compose --profile cli up

# Start full development stack
docker-compose --profile dev up
```

### 3. Production Environment

```bash
# Start production server
docker-compose --profile server up

# Start full production stack with database and cache
docker-compose --profile production up
```

## Docker Compose Profiles

The `docker-compose.yml` uses profiles to organize different deployment scenarios:

### Development Profiles

- **cli**: Orbit CLI with hot reload
- **dev**: CLI + mock service
- **mock**: Mock Anthropic service only

### Production Profiles

- **server**: Orbit server only
- **production**: Full stack (server + database + cache)
- **database**: PostgreSQL only
- **cache**: Redis only
- **proxy**: Nginx reverse proxy
- **monitoring**: Prometheus + Grafana

## Service Details

### orbit-cli
- **Purpose**: Interactive CLI development with hot reload
- **Port**: 8080
- **Volume**: Mounts entire workspace for live editing
- **Command**: `cargo watch -x "run -p orbit-cli -- --model claude-sonnet-4-6"`

### orbit-server
- **Purpose**: Production HTTP/WebSocket control plane
- **Port**: 8788
- **Health Check**: `/health` endpoint
- **Restart**: `unless-stopped`

### mock-anthropic-service
- **Purpose**: Local Anthropic-compatible mock for testing
- **Port**: 8081
- **Use Case**: Development without API costs

### postgres
- **Purpose**: Persistent data storage
- **Port**: 5432
- **Database**: `orbit`
- **User**: `orbit`

### redis
- **Purpose**: Caching and session storage
- **Port**: 6379
- **Persistence**: Data volume mounted

### nginx
- **Purpose**: Reverse proxy and SSL termination
- **Ports**: 80, 443
- **Config**: `infrastructure/nginx/`

### prometheus + grafana
- **Purpose**: Monitoring and observability
- **Ports**: 9090 (Prometheus), 3000 (Grafana)
- **Config**: `infrastructure/prometheus/`, `infrastructure/grafana/`

## Common Workflows

### Development with Hot Reload

```bash
# Start CLI development
docker-compose --profile cli up

# In another terminal, attach to the running container
docker-compose exec orbit-cli bash

# Run commands directly
docker-compose exec orbit-cli orbit --help
```

### Server Development

```bash
# Build and start server
docker-compose --profile server up

# Watch logs
docker-compose logs -f orbit-server

# Test health endpoint
curl http://localhost:8788/health
```

### Testing with Mock Service

```bash
# Start mock service
docker-compose --profile mock up

# Use mock for CLI development
ANTHROPIC_BASE_URL=http://localhost:8081 docker-compose --profile cli up
```

### Production Deployment

```bash
# Start full production stack
docker-compose --profile production up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f
```

### Monitoring Setup

```bash
# Start monitoring stack
docker-compose --profile monitoring up

# Access Grafana (admin/admin123)
open http://localhost:3000

# Access Prometheus
open http://localhost:9090
```

## Environment Variables

Key environment variables for Docker:

### API Keys
```bash
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
FRONTAL_API_KEY=frontal-...
```

### Server Configuration
```bash
ORBIT_SERVER_API_KEY=your-secret-key
ORBIT_SERVER_PORT=8788
ORBIT_SERVER_HOST=0.0.0.0
```

### Database
```bash
POSTGRES_PASSWORD=secure_password
DATABASE_URL=postgresql://orbit:password@postgres:5432/orbit
```

## Building Images

### Development Build
```bash
docker-compose build orbit-cli
```

### Production Build
```bash
docker-compose build orbit-server
```

### Multi-Platform Build
```bash
docker buildx build --platform linux/amd64,linux/arm64 -t frontal-orbit:latest .
```

## Performance Optimization

### Build Caching
The Dockerfile uses multi-stage builds with dependency caching for faster rebuilds.

### Volume Mounts
- Development: Full workspace mount for hot reload
- Production: Minimal data volumes only

### Resource Limits
Consider adding resource limits for production:

```yaml
deploy:
  resources:
    limits:
      cpus: '2'
      memory: 2G
    reservations:
      cpus: '1'
      memory: 1G
```

## Troubleshooting

### Build Issues
```bash
# Clean build
docker-compose down --volumes
docker-compose build --no-cache

# Check build logs
docker-compose build orbit-cli
```

### Permission Issues
```bash
# Fix volume permissions
docker-compose exec orbit-server sudo chown -R orbit:orbit /workspace
```

### Network Issues
```bash
# Check network connectivity
docker-compose exec orbit-cli ping postgres
docker-compose exec orbit-cli ping redis
```

### Debug Mode
```bash
# Enable debug logging
RUST_LOG=debug docker-compose --profile cli up

# Interactive shell
docker-compose exec orbit-cli bash
```

## Security Considerations

### API Keys
- Never commit API keys to version control
- Use Docker secrets or environment files
- Rotate keys regularly

### Network Security
- Use internal networks for service communication
- Expose only necessary ports
- Consider nginx proxy for SSL termination

### Container Security
- Run as non-root user (configured)
- Use minimal base images
- Regular security updates

## Production Checklist

Before deploying to production:

1. [ ] Set strong passwords in `.env`
2. [ ] Configure SSL certificates
3. [ ] Set up monitoring and alerts
4. [ ] Configure backup strategy
5. [ ] Test disaster recovery
6. [ ] Review resource limits
7. [ ] Enable log aggregation
8. [ ] Set up health checks

## Next Steps

- Configure infrastructure/nginx/ for reverse proxy
- Set up infrastructure/prometheus/ for custom metrics
- Configure infrastructure/grafana/ dashboards
- Set up CI/CD pipeline with Docker builds
