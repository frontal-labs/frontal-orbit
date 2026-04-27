# Context7 Integration Guide

This guide covers the integration of Context7 with Orbit, providing up-to-date library documentation for AI coding assistance.

## Overview

Context7 is an MCP (Model Context Protocol) server that provides real-time, version-specific documentation and code examples for popular libraries and frameworks. It ensures that AI assistants always have access to current API information rather than relying on potentially outdated training data.

## Features

- **Up-to-date Documentation**: Fetches the latest documentation directly from library sources
- **Version-Specific Examples**: Get code examples matching your specific library version
- **Automatic Library Resolution**: Automatically resolves library names to Context7 IDs
- **Cached Responses**: Improves performance with intelligent caching
- **Multi-Language Support**: Supports JavaScript, TypeScript, Python, Rust, and more

## Installation

### Prerequisites

- Node.js 18 or higher
- Orbit CLI with MCP support
- Context7 API key (free tier available)

### Quick Setup

1. **Get Context7 API Key**
   ```bash
   # Visit https://context7.com/dashboard
   # Sign up and generate an API key
   export CONTEXT7_API_KEY=your_api_key_here
   ```

2. **Run Setup Script**
   ```bash
   ./scripts/setup_context7.sh
   ```

3. **Restart Orbit**
   ```bash
   # Restart your Orbit session to load the new MCP server
   ```

### Manual Setup

1. **Install Context7 MCP Server**
   ```bash
   npx -y @upstash/context7-mcp@latest
   ```

2. **Configure Environment**
   ```bash
   export CONTEXT7_API_KEY=your_api_key_here
   ```

3. **Verify Configuration**
   Check that `context7.json` exists and `.orbit.json` contains the Context7 server configuration.

## Configuration

### context7.json

The main configuration file for Context7:

```json
{
  "name": "context7",
  "version": "1.0.0",
  "server": {
    "command": "npx",
    "args": ["-y", "@upstash/context7-mcp@latest"],
    "env": {
      "CONTEXT7_API_KEY": "${CONTEXT7_API_KEY}",
      "NODE_ENV": "production"
    },
    "timeout": 30,
    "auto_start": true,
    "enabled": true
  }
}
```

### .orbit.json Integration

Context7 is automatically configured in the MCP servers section:

```json
{
  "mcp": {
    "auto_start": ["filesystem", "context7"],
    "servers": {
      "context7": {
        "command": "npx",
        "args": ["-y", "@upstash/context7-mcp@latest"],
        "env": {
          "CONTEXT7_API_KEY": "${CONTEXT7_API_KEY}"
        },
        "timeout": 30,
        "auto_start": true
      }
    }
  }
}
```

## Usage

### Basic Usage

Add "use context7" to your prompts to automatically fetch up-to-date documentation:

```bash
orbit prompt "Create a Next.js middleware for JWT authentication. use context7"
```

### Available Tools

#### resolve_library_id
Resolves library names to Context7 library IDs:
```bash
orbit prompt "Resolve library ID for React hooks. use context7"
```

#### get_library_docs
Fetches documentation for a specific library:
```bash
orbit prompt "Get documentation for React useState hook. use context7"
```

#### search_libraries
Searches for available libraries:
```bash
orbit prompt "Search for Vue.js libraries. use context7"
```

### Examples

#### React Development
```bash
# Get latest React hooks documentation
orbit prompt "Show me how to use useEffect and useCallback in React 18. use context7"

# Create a custom hook with current best practices
orbit prompt "Create a custom hook for API calls with loading states. use context7"
```

#### Next.js Development
```bash
# Next.js 14+ features
orbit prompt "Create a Next.js 14 app router with server components. use context7"

# Middleware configuration
orbit prompt "Set up Next.js middleware for authentication. use context7"
```

#### TypeScript
```bash
# TypeScript 5.x features
orbit prompt "Show me TypeScript 5.x utility types examples. use context7"

# Advanced type patterns
orbit prompt "Create conditional types for API responses. use context7"
```

## Supported Libraries

Context7 supports a wide range of popular libraries:

### Frontend Frameworks
- React (all versions)
- Next.js
- Vue.js
- Angular
- Svelte

### Backend Frameworks
- Express.js
- Fastify
- Koa
- NestJS

### Database Libraries
- Prisma
- Drizzle ORM
- TypeORM
- Sequelize

### Testing Libraries
- Jest
- Vitest
- Cypress
- Playwright

### Build Tools
- Vite
- Webpack
- Rollup
- esbuild

## Configuration Options

### Environment Variables

- `CONTEXT7_API_KEY`: Your Context7 API key (required)
- `NODE_ENV`: Node environment (default: production)

### Server Configuration

- `timeout`: Request timeout in seconds (default: 30)
- `auto_start`: Automatically start server with Orbit (default: true)
- `max_retries`: Maximum retry attempts (default: 3)
- `retry_delay`: Delay between retries in ms (default: 1000)

### Cache Configuration

- `cache.enabled`: Enable response caching (default: true)
- `cache.ttl_seconds`: Cache TTL in seconds (default: 300)
- `cache.max_size_mb`: Maximum cache size in MB (default: 100)

## Troubleshooting

### Common Issues

#### API Key Not Found
```bash
# Check if API key is set
echo $CONTEXT7_API_KEY

# Set the API key
export CONTEXT7_API_KEY=your_key_here
```

#### Server Not Starting
```bash
# Check MCP server status
orbit mcp status context7

# Restart the server
orbit mcp restart context7
```

#### Documentation Not Found
```bash
# Check available libraries
orbit prompt "Search for available libraries. use context7"

# Verify library name spelling
orbit prompt "Resolve library ID for [library_name]. use context7"
```

### Debug Mode

Enable debug logging for troubleshooting:

```bash
# Set debug environment variable
export RUST_LOG=orbit_mcp=debug

# Run Orbit with debug logging
orbit --log-level debug
```

### Health Check

Verify Context7 server health:

```bash
# Check server health
orbit mcp health context7

# Test server connection
orbit mcp test context7
```

## Best Practices

### 1. Always Specify Versions
```bash
# Good
orbit prompt "Show me React 18 hooks documentation. use context7"

# Better
orbit prompt "Show me React 18.2.0 useEffect hook documentation. use context7"
```

### 2. Use Specific Library Names
```bash
# Good
orbit prompt "Next.js app router documentation. use context7"

# Better
orbit prompt "Next.js 14 app router server components. use context7"
```

### 3. Combine with Orbit Tools
```bash
# Create files with up-to-date documentation
orbit prompt "Create a React component using latest hooks patterns. use context7"
```

### 4. Cache Management
```bash
# Clear cache if documentation seems outdated
orbit mcp cache clear context7
```

## Performance Optimization

### Response Caching
Context7 automatically caches documentation responses to improve performance. Cache is invalidated based on TTL or manual clearing.

### Rate Limiting
Free tier has rate limits. Consider upgrading for production use or implement request batching.

### Prefetching
Frequently used libraries are automatically prefetched during Orbit startup.

## Security

### API Key Security
- Store API keys in environment variables
- Never commit API keys to version control
- Use different keys for development and production

### Network Security
Context7 only communicates with official Context7 endpoints:
- `https://mcp.context7.com/mcp`
- `https://context7.com`

### Content Sanitization
All documentation responses are sanitized before being provided to the AI model.

## Integration Examples

### Web Development Workflow
```bash
# Start development with latest documentation
orbit prompt "Set up a new Next.js project with TypeScript and Tailwind. use context7"

# Add features with current best practices
orbit prompt "Add authentication using NextAuth.js v5. use context7"
```

### API Development
```bash
# Create API with latest patterns
orbit prompt "Create a REST API using Fastify with TypeScript. use context7"

# Add database integration
orbit prompt "Add Prisma ORM with PostgreSQL integration. use context7"
```

### Testing Setup
```bash
# Configure testing with current tools
orbit prompt "Set up Vitest with React Testing Library. use context7"

# Add E2E testing
orbit prompt "Configure Playwright for E2E testing. use context7"
```

## Contributing

To contribute to Context7 integration:

1. Report issues on the Orbit GitHub repository
2. Submit pull requests for improvements
3. Update documentation for new features

## Resources

- [Context7 Official Documentation](https://github.com/upstash/context7)
- [Context7 Dashboard](https://context7.com/dashboard)
- [Orbit MCP Guide](./MCP.md)
- [Context7 API Reference](https://context7.com/docs/api)

## Support

For Context7-specific issues:
- Context7 GitHub: https://github.com/upstash/context7/issues
- Context7 Discord: [link to Discord]

For Orbit integration issues:
- Orbit GitHub: https://github.com/frontal-labs/frontal-orbit/issues
- Orbit Documentation: ./README.md
