# Quick Start: Orbit Hosted Services

This guide helps you set up the complete Orbit autonomous engineering platform with Slack bot and GitHub integration.

## Prerequisites

- Docker and Docker Compose installed
- Node.js 18+ for Slack bot development
- GitHub organization with admin access
- Slack workspace with admin access

## Step 1: Configure Environment

Create a `.env` file in the root directory:

```bash
# Database Configuration
DATABASE_URL=postgresql://orbit:orbit_password@localhost:5432/orbit_db
REDIS_URL=redis://:redis_password@localhost:6379

# Memory Configuration (Pinecone)
ORBIT_MEMORY_PINECONE_URL=https://your-pinecone-index.pinecone.io
ORBIT_MEMORY_PINECONE_API_KEY=your-pinecone-api-key
ORBIT_MEMORY_PINECONE_NAMESPACE=orbit-memory

# GitHub App Configuration
GITHUB_APP_ID=your-github-app-id
GITHUB_PRIVATE_KEY_PATH=/path/to/github/private/key.pem

# Slack Bot Configuration
SLACK_BOT_TOKEN=xoxb-your-slack-bot-token
SLACK_SIGNING_SECRET=your-slack-signing-secret

# Server Configuration
ORBIT_SERVER_URL=https://your-orbit-server.com
WEBHOOK_SECRET=your-webhook-secret
```

## Step 2: Create GitHub App

1. Go to GitHub Settings > Developer settings > GitHub Apps
2. Click "New GitHub App"
3. Use the configuration in `.github/app.yml`
4. Install the app to your organization
5. Note the App ID and generate/download private key
6. Update your `.env` file with the GitHub App credentials

## Step 3: Create Slack App

1. Go to [Slack API Apps](https://api.slack.com/apps)
2. Create a new app with settings from `extensions/orbit-slack/INSTALL.md`
3. Install the app to your workspace
4. Update your `.env` file with Slack credentials

## Step 4: Start Services

```bash
# Start all hosted services
docker-compose -f docker-compose.hosted.yml up -d

# Check service status
docker-compose -f docker-compose.hosted.yml ps

# View logs
docker-compose -f docker-compose.hosted.yml logs -f orbit-server
```

## Step 5: Verify Integration

### Test Slack Bot
In your Slack workspace, try:
- `/orbit ask "What can you do?"`
- `/orbit status`
- `/orbit help`

### Test GitHub Integration
1. Create a test repository
2. Make a commit and push
3. Check that Orbit processes the webhook
4. Verify autonomous actions (branch creation, PR creation, etc.)

### Test Autonomous Capabilities
Try these commands:
- `/orbit fix issue https://github.com/your-org/your-repo/issues/1`
- `/orbit test all`
- `/orbit deploy staging`

## Architecture Overview

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Slack Bot    │    │  Orbit Server   │    │  GitHub App    │
│               │◄──►│               │◄──►│               │
│  - Commands   │    │  - Webhook    │    │  - Events      │
│  - Events     │    │  - API         │    │  - Permissions  │
│  - Uploads    │    │  - Storage      │    │  - Webhooks     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Service URLs

- **Orbit Server**: http://localhost:8788
- **Health Check**: http://localhost:8788/health
- **GitHub Webhook**: https://your-orbit-server.com/webhook/github
- **Slack Events**: Received via Socket Mode

## Monitoring

Check these endpoints:
- Server logs: `docker-compose logs -f orbit-server`
- Slack bot logs: `docker-compose logs -f orbit-slack`
- Database status: Connect to PostgreSQL container
- API health: `curl http://localhost:8788/health`

## Troubleshooting

**Services won't start:**
- Check all environment variables are set
- Verify Docker Compose configuration
- Check port conflicts (8788, 5432, 6379)

**Slack bot not responding:**
- Verify `SLACK_BOT_TOKEN` and `SLACK_SIGNING_SECRET`
- Check bot is invited to channels
- Review Slack app permissions

**GitHub webhooks not working:**
- Verify webhook URL is accessible from GitHub
- Check GitHub App permissions match `.github/app.yml`
- Review Orbit server logs for webhook processing

## Next Steps

After setup:
1. Configure autonomous workflows in `docs/autonomous-engineering-implementation-plan.md`
2. Set up monitoring and alerting
3. Configure production deployment (separate from development)
4. Train the system with your organization's patterns

## Support

- **Documentation**: See `docs/` directory
- **Issues**: Create issues in this repository
- **Community**: Join discussions in GitHub Discussions

For detailed setup instructions, see:
- `extensions/orbit-slack/INSTALL.md` - Slack bot setup
- `docs/autonomous-engineering-implementation-plan.md` - Architecture
- `.github/app.yml` - GitHub App configuration
