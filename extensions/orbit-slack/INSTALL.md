# Orbit Slack Bot Installation Guide

## Overview

The Orbit Slack bot provides autonomous engineering capabilities directly within Slack, allowing teams to:
- Create and manage branches
- Make commits and push changes
- Run tests and create pull requests
- Deploy code to environments
- Monitor and fix issues autonomously

## Prerequisites

1. **Node.js 20+** installed on the deployment machine
2. **Orbit Server** running and accessible from the internet
3. **Slack App** created in your Slack workspace
4. **GitHub App** configured with appropriate permissions

## Installation Steps

### 1. Create Slack App

1. Go to [Slack API Apps](https://api.slack.com/apps)
2. Click "Create New App"
3. Configure app settings:
   - **App Name**: "Orbit Autonomous Engineering"
   - **Description**: "AI-powered autonomous engineering platform"
   - **Development Workspace**: Select your target workspace

### 2. Configure OAuth & Permissions

Add these **Bot Token Scopes**:
- `channels:read` - Read channel messages
- `channels:write` - Post messages to channels
- `chat:write` - Send direct messages
- `commands` - Handle slash commands
- `users:read` - Read user information
- `files:write` - Upload files to messages

Add these **Event Subscriptions**:
- `message.channels` - Channel messages
- `app_mention` - Bot mentions
- `message.groups` - Private channel messages

### 3. Configure Interactive Components

Enable these **Interactive Components**:
- `commands` - Slash command handling
- `message` - Message interactions

### 4. Install App to Workspace

1. Go to "Install App" in your Slack app settings
2. Select the workspace(s) to install
3. Authorize the requested permissions
4. Note the **Bot User OAuth Token** (starts with `xoxb-`)

## 3. Configure Orbit Slack Bot

### Environment Variables

Create a `.env` file in the `extensions/orbit-slack` directory:

```bash
# Slack App Configuration
SLACK_BOT_TOKEN=xoxb-your-bot-token-here
SLACK_SIGNING_SECRET=your-signing-secret-here
SLACK_APP_ID=A0123456789012

# Orbit Server Configuration
ORBIT_API_URL=https://your-orbit-server.com
ORBIT_API_KEY=your-orbit-api-key-here

# Optional: GitHub Integration
GITHUB_APP_ID=your-github-app-id
GITHUB_PRIVATE_KEY_PATH=/path/to/github/private/key.pem
```

Set the same shared secret value as `ORBIT_SERVER_API_KEY` on the hosted server and
`ORBIT_API_KEY` on the Slack connector so authenticated control-plane calls succeed.

### 4. Install Dependencies and Build

```bash
cd extensions/orbit-slack
bun install
bun run build
```

### 5. Start the Slack Bot

```bash
# Development
bun run dev

# Production
bun start
```

## Slash Commands

Once installed, users can interact with Orbit using these commands:

### Core Commands
- `/orbit ask <question>` - Ask the AI assistant questions
- `/orbit task <description>` - Create and execute autonomous tasks
- `/orbit status` - Check current system status
- `/orbit help` - Show available commands

### Git Operations
- `/orbit branch create <name>` - Create new branch
- `/orbit branch switch <name>` - Switch to existing branch
- `/orbit commit <message>` - Commit current changes
- `/orbit pr create <title>` - Create pull request
- `/orbit pr merge <number>` - Merge pull request
- `/orbit deploy <env>` - Deploy to environment

### Autonomous Workflows
- `/orbit fix issue <url>` - Automatically analyze and fix GitHub issue
- `/orbit review pr <number>` - Review pull request changes
- `/orbit test all` - Run full test suite and fix failures
- `/orbit deploy staging` - Deploy current branch to staging

## GitHub Integration

### 1. Create GitHub App

Use the `.github/app.yml` configuration to create a GitHub App with:
- Repository write permissions
- Issue and PR management
- Check run and status permissions
- Webhook events for push, PR, issues

### 2. Configure Webhook URL

Set the GitHub App webhook URL to:
```
https://your-orbit-server.com/webhook/github
```

### 3. Install GitHub App

1. Install the GitHub App to your organization
2. Note the **App ID** and **Private Key**
3. Configure environment variables in your Orbit server

## Deployment Options

### Option A: Integrated Deployment
- Orbit server handles both Slack and GitHub webhooks
- Single deployment manages both integrations
- Recommended for production

### Option B: Separate Services  
- Slack bot runs independently
- GitHub webhooks go to separate Orbit server
- Good for development and testing

## Security Considerations

1. **Verify Requests**: Always validate Slack request signatures
2. **Scope Permissions**: Use minimum required scopes
3. **Secret Management**: Store tokens securely, use environment variables
4. **Access Control**: Implement proper authorization checks

## Troubleshooting

### Common Issues

**Bot not responding:**
- Check `SLACK_BOT_TOKEN` is correct
- Verify bot is invited to the channel
- Check server logs for errors

**GitHub webhooks not working:**
- Verify webhook URL is accessible
- Check GitHub App permissions
- Look for webhook delivery failures

**Commands not working:**
- Ensure slash commands are registered in Slack app
- Check bot has required scopes
- Verify command syntax

### Monitoring

Monitor these metrics:
- Response times and success rates
- Error rates and types
- Command usage patterns
- Autonomous task completion rates

## Development Setup

For local development:

```bash
# Clone the extension
git clone <your-repo>
cd extensions/orbit-slack

# Install dependencies
bun install

# Configure environment
cp .env.example .env
# Edit .env with your tokens

# Start development server
bun run dev
```

The bot will connect to your Slack workspace and be ready for testing.
