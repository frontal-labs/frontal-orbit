# Development Environment Setup

## Quick Start

### Using Nix (Recommended)
```bash
# Enter development environment
nix develop

# Build the project
cargo build --workspace

# Run tests
cargo test --workspace

# Start the CLI
cargo run --bin orbit
```

### Using Docker Compose
```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f orbit

# Stop services
docker-compose down
```

### Development with hot reload
```bash
# Start development environment with hot reload
docker-compose --profile dev up orbit-dev

# This will automatically rebuild and restart when you save changes
```

## Environment Variables

Create a `.env` file for local development:

```bash
# AI Provider
ANTHROPIC_API_KEY=sk-ant-...

# Database (for local development)
DATABASE_URL=sqlite:///tmp/orbit_dev.db

# Optional: PostgreSQL for full server mode
DATABASE_URL=postgresql://orbit:orbit_password@localhost:5432/orbit_db

# Webhook secret for server mode
WEBHOOK_SECRET=your_webhook_secret_here
```

## Pre-commit Hooks

Install pre-commit hooks:

```bash
# Install pre-commit
pip install pre-commit

# Install hooks
pre-commit install

# Run hooks manually
pre-commit run --all-files
```

## Development Commands

```bash
# Build
cargo build --workspace

# Build release
cargo build --release --workspace

# Run tests
cargo test --workspace

# Run with logging
RUST_LOG=debug cargo run --bin orbit

# Check code
cargo check --workspace

# Format code
cargo fmt --all

# Lint code
cargo clippy --workspace -- -D warnings

# Run doctor check
cargo run --bin orbit -- doctor

# Generate documentation
cargo doc --workspace --no-deps --document-private-items

# Audit dependencies
cargo audit
```

## Services Overview

### Core Services
- **orbit**: Main Orbit service
- **postgres**: PostgreSQL database for structured memory
- **redis**: Redis for caching and session management
- **qdrant**: Vector database for semantic search
- **webhook-server**: Webhook receiver for external events

### Development Services
- **orbit-dev**: Development environment with hot reload

## Database Setup

### PostgreSQL
```bash
# Connect to database
docker-compose exec postgres psql -U orbit -d orbit_db

# View tables
\dt

# View schema
\d+ tasks
```

### Redis
```bash
# Connect to Redis
docker-compose exec redis redis-cli -a redis_password

# View keys
KEYS *
```

### Qdrant
```bash
# View collections
curl http://localhost:6333/collections

# Health check
curl http://localhost:6333/health
```

## Testing

```bash
# Run all tests
cargo test --workspace

# Run specific test
cargo test --package orbit-cli test_name

# Run tests with output
cargo test --workspace -- --nocapture

# Run mock parity harness
./scripts/run_mock_parity_harness.sh
```

## Troubleshooting

### Common Issues

1. **Permission denied errors**: Make sure your user has permissions for the workspace directory
2. **Database connection errors**: Check that database services are running and accessible
3. **Build failures**: Ensure all dependencies are installed and Rust toolchain is up to date

### Health Checks

```bash
# Check Orbit health
docker-compose exec orbit orbit doctor

# Check service status
docker-compose ps

# View logs
docker-compose logs [service_name]
```

### Reset Environment

```bash
# Stop all services
docker-compose down

# Remove volumes (this will delete all data!)
docker-compose down -v

# Rebuild and start
docker-compose up --build
```
