# MCP (Model Context Protocol) Guide

This guide covers the Model Context Protocol (MCP) integration in Orbit, including setup, configuration, and usage.

## Overview

MCP is a protocol for connecting AI models to external tools and data sources. Orbit includes full MCP support, allowing you to:

- Connect to MCP servers for additional tools
- Use MCP-provided data sources
- Integrate with external services
- Extend tool capabilities beyond built-in tools

## MCP Architecture

### Components

- **MCP Client**: Built into Orbit, handles communication with servers
- **MCP Servers**: External processes that provide tools and resources
- **MCP Protocol**: JSON-RPC based communication protocol
- **MCP Registry**: Directory of available MCP servers

### Communication Flow

```
Orbit CLI -> MCP Client -> MCP Server -> External Service
```

## Built-in MCP Servers

### Filesystem Server

Provides enhanced file system operations:

```bash
# Start filesystem server
orbit mcp start filesystem

# Use filesystem tools
/filesystem/read /path/to/file
/filesystem/write /path/to/file "content"
/filesystem/list /path/to/directory
/filesystem/watch /path/to/directory
```

### GitHub Server

Integrates with GitHub API:

```bash
# Start GitHub server
orbit mcp start github

# Use GitHub tools
/github/repo list
/github/issue list owner/repo
/github/pull list owner/repo
/github/file get owner/repo path/to/file
```

### Database Server

Database connectivity:

```bash
# Start database server
orbit mcp start database

# Use database tools
/database/connect postgresql://user:pass@localhost/db
/database/query "SELECT * FROM users"
/database/schema show
```

## MCP Configuration

### Server Configuration

Configure MCP servers in `.orbit.json`:

```json
{
  "mcp": {
    "servers": {
      "filesystem": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/gabriel/Downloads"],
        "env": {
          "NODE_ENV": "production"
        },
        "timeout": 30,
        "auto_start": true,
        "enabled": true
      },
      "github": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-github"],
        "env": {
          "GITHUB_PERSONAL_ACCESS_TOKEN": "${GITHUB_TOKEN}"
        },
        "timeout": 60,
        "auto_start": false,
        "enabled": true
      },
      "postgres": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-postgres"],
        "env": {
          "POSTGRES_CONNECTION_STRING": "postgresql://user:pass@localhost/db"
        },
        "timeout": 30,
        "auto_start": false,
        "enabled": true
      }
    }
  }
}
```

### Global MCP Settings

```json
{
  "mcp": {
    "servers_dir": "~/.orbit/mcp-servers",
    "timeout": 30,
    "auto_start": ["filesystem"],
    "enabled": true,
    "max_concurrent_servers": 10,
    "health_check_interval": 60,
    "restart_on_failure": true,
    "log_level": "info"
  }
}
```

## MCP Commands

### Server Management

```bash
# List available MCP servers
orbit mcp list

# List running servers
orbit mcp list --running

# Start specific server
orbit mcp start filesystem

# Stop server
orbit mcp stop filesystem

# Restart server
orbit mcp restart filesystem

# Show server status
orbit mcp status filesystem

# Show server details
orbit mcp show filesystem
```

### Server Configuration

```bash
# Configure server
orbit mcp config filesystem --timeout 60 --auto-start

# Add new server
orbit mcp add my-server --command "npx" --args "-y" "@my/mcp-server"

# Remove server
orbit mcp remove my-server

# Test server connection
orbit mcp test filesystem
```

### Tool Management

```bash
# List available MCP tools
orbit mcp tools

# List tools from specific server
orbit mcp tools filesystem

# Show tool schema
orbit mcp tool schema filesystem/read

# Test tool
orbit mcp tool test filesystem/read --arg path="/tmp/test"
```

## Using MCP Tools

### Filesystem Operations

```bash
# Read file
orbit prompt "Use filesystem to read /etc/hosts"

# Write file
orbit prompt "Use filesystem to write hello world to /tmp/test.txt"

# List directory
orbit prompt "Use filesystem to list contents of /Users/gabriel/Downloads"

# Watch directory
orbit prompt "Use filesystem to watch /tmp for changes"
```

### GitHub Integration

```bash
# List repositories
orbit prompt "Use GitHub to list my repositories"

# Get repository information
orbit prompt "Use GitHub to get information about orbit-org/orbit-cli"

# List issues
orbit prompt "Use GitHub to list open issues in orbit-org/orbit-cli"

# Create issue
orbit prompt "Use GitHub to create an issue in orbit-org/orbit-cli with title 'Bug found'"
```

### Database Operations

```bash
# Connect to database
orbit prompt "Use database to connect to postgresql://user:pass@localhost/mydb"

# Run query
orbit prompt "Use database to run query SELECT * FROM users LIMIT 10"

# Show schema
orbit prompt "Use database to show the schema of the users table"
```

## Custom MCP Servers

### Creating a Custom Server

1. **Initialize MCP server project**
   ```bash
   mkdir my-mcp-server
   cd my-mcp-server
   npm init -y
   npm install @modelcontextprotocol/sdk
   ```

2. **Create server implementation**
   ```javascript
   // server.js
   const { Server } = require('@modelcontextprotocol/sdk/server/index.js');
   const { StdioServerTransport } = require('@modelcontextprotocol/sdk/server/stdio.js');
   
   const server = new Server({
     name: 'my-mcp-server',
     version: '1.0.0',
   }, {
     capabilities: {
       tools: {},
     },
   });
   
   // Define tools
   server.setRequestHandler('tools/list', async () => ({
     tools: [
       {
         name: 'my_tool',
         description: 'A custom tool',
         inputSchema: {
           type: 'object',
           properties: {
             input: { type: 'string' }
           }
         }
       }
     ]
   }));
   
   // Handle tool calls
   server.setRequestHandler('tools/call', async (request) => {
     const { name, arguments: args } = request.params;
     
     if (name === 'my_tool') {
       const result = `Processed: ${args.input}`;
       return {
         content: [{ type: 'text', text: result }]
       };
     }
     
     throw new Error('Unknown tool');
   });
   
   // Start server
   const transport = new StdioServerTransport();
   server.connect(transport);
   ```

3. **Configure in Orbit**
   ```json
   {
     "mcp": {
       "servers": {
         "my-server": {
           "command": "node",
           "args": ["server.js"],
           "timeout": 30,
           "auto_start": false
         }
       }
     }
   }
   ```

### Python MCP Server

```python
# server.py
import asyncio
import json
from mcp.server import Server
from mcp.server.stdio import stdio_server

app = Server('my-mcp-server')

@app.list_tools()
async def list_tools():
    return [
        {
            'name': 'my_tool',
            'description': 'A custom tool',
            'inputSchema': {
                'type': 'object',
                'properties': {
                    'input': {'type': 'string'}
                }
            }
        }
    ]

@app.call_tool()
async def call_tool(name, arguments):
    if name == 'my_tool':
        result = f"Processed: {arguments['input']}"
        return {
            'content': [{'type': 'text', 'text': result}]
        }
    
    raise ValueError(f"Unknown tool: {name}")

async def main():
    async with stdio_server() as (read_stream, write_stream):
        await app.run(read_stream, write_stream)

if __name__ == '__main__':
    asyncio.run(main())
```

## MCP Protocol Details

### Message Format

MCP uses JSON-RPC 2.0 over stdio:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}
```

### Supported Methods

#### tools/list
List available tools from a server.

#### tools/call
Execute a tool with given arguments.

#### resources/list
List available resources.

#### resources/read
Read a resource's contents.

#### prompts/list
List available prompts.

#### prompts/get
Get a specific prompt.

### Error Handling

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32601,
    "message": "Method not found",
    "data": {}
  }
}
```

## MCP Security

### Sandboxing

MCP servers run in isolated processes with:

- Limited file system access
- Restricted network access
- Controlled environment variables
- Resource limits (CPU, memory)

### Authentication

```json
{
  "mcp": {
    "servers": {
      "secure-server": {
        "command": "npx",
        "args": ["-y", "@secure/mcp-server"],
        "env": {
          "API_TOKEN": "${SECURE_API_TOKEN}",
          "SERVER_CERT": "/path/to/cert.pem"
        },
        "permissions": {
          "network": ["api.example.com"],
          "filesystem": ["/tmp"],
          "environment": ["API_TOKEN"]
        }
      }
    }
  }
}
```

### Best Practices

1. **Validate inputs** - Always validate tool arguments
2. **Use secure defaults** - Don't expose sensitive data
3. **Limit permissions** - Grant minimum necessary permissions
4. **Monitor usage** - Track server resource usage
5. **Update regularly** - Keep MCP servers updated

## Troubleshooting

### Common Issues

1. **Server fails to start**
   - Check command and arguments
   - Verify environment variables
   - Check server dependencies

2. **Tools not available**
   - Verify server is running
   - Check tool registration
   - Review server logs

3. **Connection timeout**
   - Increase timeout value
   - Check server responsiveness
   - Verify network connectivity

### Debug Mode

```bash
# Enable MCP debug logging
RUST_LOG=orbit_mcp=debug orbit mcp list

# Test server connection
orbit mcp test filesystem --debug

# Show server logs
orbit mcp logs filesystem
```

### Health Checks

```bash
# Check server health
orbit mcp health filesystem

# Check all servers
orbit mcp health --all

# Continuous monitoring
orbit mcp health --watch
```

## Performance Optimization

### Server Configuration

```json
{
  "mcp": {
    "servers": {
      "optimized-server": {
        "command": "npx",
        "args": ["-y", "@optimized/mcp-server"],
        "timeout": 60,
        "max_concurrent_requests": 10,
        "connection_pool_size": 5,
        "cache_ttl": 300,
        "compression": true
      }
    }
  }
}
```

### Caching

```bash
# Enable tool result caching
orbit mcp config filesystem --cache-enabled true

# Set cache TTL
orbit mcp config filesystem --cache-ttl 300

# Clear cache
orbit mcp cache clear filesystem
```

### Load Balancing

```json
{
  "mcp": {
    "load_balancing": {
      "strategy": "round_robin",
      "health_check_interval": 30,
      "failover_timeout": 10
    }
  }
}
```

## MCP Ecosystem

### Official Servers

- **@modelcontextprotocol/server-filesystem** - File system operations
- **@modelcontextprotocol/server-github** - GitHub integration
- **@modelcontextprotocol/server-postgres** - PostgreSQL database
- **@modelcontextprotocol/server-slack** - Slack integration
- **@modelcontextprotocol/server-puppeteer** - Web automation

### Community Servers

- **@mcp/server-kubernetes** - Kubernetes operations
- **@mcp/server-aws** - AWS services
- **@mcp/server-docker** - Docker container management
- **@mcp/server-redis** - Redis database operations
- **@mcp/server-elasticsearch** - Elasticsearch operations

### Finding Servers

```bash
# Search MCP registry
orbit mcp search filesystem

# List popular servers
orbit mcp list --popular

# Show server details
orbit mcp info @modelcontextprotocol/server-filesystem
```

## MCP Roadmap

### Upcoming Features

- **MCP 2.0** - Enhanced protocol with streaming support
- **Hot reloading** - Reload servers without restart
- **Server federation** - Connect multiple MCP instances
- **Resource sharing** - Share resources between servers
- **Advanced security** - Enhanced authentication and authorization

### Protocol Enhancements

- **Streaming responses** - Support for streaming tool results
- **Batch operations** - Execute multiple tools in parallel
- **Event notifications** - Server-to-client event streaming
- **Tool composition** - Combine multiple tools into workflows

## Integration Examples

### Web Development Workflow

```bash
# Start relevant MCP servers
orbit mcp start filesystem
orbit mcp start github

# Use in development
orbit prompt "Use filesystem to create a new React component and GitHub to create a pull request"
```

### Data Analysis Workflow

```bash
# Start database and analysis servers
orbit mcp start postgres
orbit mcp start pandas

# Analyze data
orbit prompt "Use database to query user data and pandas to analyze trends"
```

### DevOps Workflow

```bash
# Start infrastructure servers
orbit mcp start kubernetes
orbit mcp start docker

# Deploy application
orbit prompt "Use Kubernetes to deploy the application and Docker to build the container"
```

This MCP guide provides comprehensive coverage of MCP integration in Orbit, from basic usage to advanced custom server development.
