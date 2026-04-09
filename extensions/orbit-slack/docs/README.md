# Orbit Slack Extension Documentation

## Overview

The Orbit Slack extension provides autonomous engineering capabilities directly within Slack, enabling teams to create, manage, and monitor coding tasks through natural language commands.

## Features

- **Task Creation**: Create hosted Orbit tasks from Slack messages and slash commands
- **Real-time Updates**: Stream task progress and updates directly into Slack threads
- **Approval Workflows**: Handle orphaned hosted-agent approvals with interactive buttons
- **Policy Inspection**: Query and preview orphan policies for different task configurations
- **Task Recovery**: Automatically recover active Slack tasks and approvals on startup

## Architecture

### Core Components

- **Slack Interface** (`src/slack.ts`): Socket-mode client, command handling, thread management
- **Orbit API Client** (`src/api-client.ts`): HTTP client for tasks, approvals, and callbacks
- **Event Stream Client** (`src/orbit-events.ts`): WebSocket client for real-time task events
- **Configuration** (`src/config.ts`, `src/env.ts`): Environment validation and settings
- **Type Definitions** (`src/types.ts`): Slack and Orbit API type definitions

### Data Flow

1. Slack command/message received
2. Task created via Orbit API
3. WebSocket subscription to task events
4. Real-time updates streamed back to Slack
5. Interactive elements for approvals and actions

## Configuration

### Environment Variables

Required environment variables:

```bash
# Slack Configuration
SLACK_BOT_TOKEN=xoxb-your-bot-token
SLACK_APP_TOKEN=xapp-your-app-token  
SLACK_SIGNING_SECRET=your-signing-secret

# Orbit Server
ORBIT_API_URL=http://127.0.0.1:8788
ORBIT_API_TIMEOUT=30000

# Application
NODE_ENV=development
LOG_LEVEL=info
PORT=3000
```

Optional variables:

```bash
# GitHub Integration
GITHUB_TOKEN=your-github-token

# Monitoring
SENTRY_DSN=your-sentry-dsn

# Performance
MAX_CONCURRENT_TASKS=10
TASK_TIMEOUT=3600000
HEALTH_CHECK_INTERVAL=30000
```

## Commands

### Slash Commands

- `/ai <prompt>`: Create a hosted Orbit task with the given prompt
- `/ai policy orphans`: Display default orphan policy and rule set
- `/ai policy orphans repo=<repo> source=<source> priority=<priority>`: Preview effective policy

### Message Handling

- Non-bot messages automatically create hosted Orbit tasks
- Approval buttons in task threads resolve orphaned approvals
- Interactive elements provide task control actions

### Examples

```bash
# Basic task creation
/ai Fix the login bug in auth service

# Policy inspection
/ai policy orphans
/ai policy orphans repo=myorg/myapp source=api
/ai policy orphans repo=myorg/myapp source=slack priority=high
```

## Development

### Prerequisites

- Node.js 20+
- Bun package manager
- Running Orbit server
- Slack app with Socket Mode enabled

### Setup

```bash
# Install dependencies
bun install

# Configure environment
cp .env.example .env
# Edit .env with your configuration

# Start development server
bun run dev
```

### Useful Commands

```bash
# Build the application
bun run build

# Run tests
bun test
bun run test:coverage
bun run test:watch

# Lint and format
bun run lint
bun run lint:fix
bun run format

# Sync Orbit event types
bun run sync:orbit-events
```

### Testing

The extension includes comprehensive test coverage:

- Unit tests for core functionality
- Integration tests for Slack interactions
- Mock Orbit server for testing
- Coverage reports generated in `coverage/`

## Deployment

### Docker Deployment

```bash
# Build the Docker image
docker build -t orbit-slack .

# Run the container
docker run -p 3000:3000 --env-file .env orbit-slack
```

### Production Considerations

- Use `NODE_ENV=production`
- Configure proper logging levels
- Set up monitoring and alerting
- Use secure secret management
- Configure health checks

## Slack App Setup

### Required Permissions

Bot Token Scopes:
- `commands` - Handle slash commands
- `chat:write` - Send messages
- `chat:write.public` - Post in public channels
- `users:read` - Read user information
- `channels:read` - Read channel information

Event Subscriptions:
- `message.channels` - Channel messages
- `app_mention` - Bot mentions
- `message.groups` - Private channel messages

### Interactive Components

Enable:
- `commands` - Slash command handling
- `message` - Message interactions

## Troubleshooting

### Common Issues

**Bot not responding:**
- Verify `SLACK_BOT_TOKEN` is correct
- Ensure bot is invited to the channel
- Check server logs for errors

**Tasks not creating:**
- Confirm Orbit server is accessible
- Check `ORBIT_API_URL` configuration
- Verify API authentication

**WebSocket connection issues:**
- Check firewall settings
- Verify Socket Mode is enabled
- Review `SLACK_APP_TOKEN` configuration

### Debug Mode

Enable debug logging:

```bash
LOG_LEVEL=debug bun run dev
```

### Health Checks

The extension exposes a health endpoint:

```bash
curl http://localhost:3000/health
```

## Security

### Best Practices

1. **Token Security**: Store tokens in environment variables, never in code
2. **Request Validation**: Verify Slack request signatures
3. **Least Privilege**: Use minimum required scopes
4. **Secret Rotation**: Regularly rotate tokens and secrets
5. **Access Control**: Implement proper authorization checks

### Request Verification

All incoming Slack requests are verified using the signing secret to prevent unauthorized requests.

## Monitoring

### Metrics to Monitor

- Response times and success rates
- Error rates and types
- Command usage patterns
- Task completion rates
- WebSocket connection health

### Logging

The extension uses structured logging with Winston:

- Info level for normal operations
- Error level for failures
- Debug level for detailed troubleshooting
- Configurable log levels

## API Reference

### Slack Endpoints

The extension provides several HTTP endpoints:

- `POST /slack/events` - Slack event callbacks
- `POST /slack/commands` - Slash command handling
- `POST /slack/interactive` - Interactive component responses
- `GET /health` - Health check endpoint

### Orbit API Integration

The extension integrates with several Orbit API endpoints:

- Task creation and management
- Approval workflows
- Event streaming
- Policy inspection

## Contributing

### Development Workflow

1. Fork the repository
2. Create a feature branch
3. Make changes with tests
4. Ensure all tests pass
5. Submit a pull request

### Code Style

- Use TypeScript for all new code
- Follow the existing code style
- Add JSDoc comments for public functions
- Include tests for new features

## License

MIT License - see LICENSE file for details.
