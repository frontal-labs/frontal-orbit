# Orbit Slack Connector

TypeScript Slack connector for Orbit hosted tasks. It creates tasks from Slack, subscribes to Orbit task events over WebSocket, reconstructs Slack routing from Orbit task snapshots and event payloads, renders approval workflows, and exposes lightweight operator commands such as orphan policy inspection.

## Current Capabilities

- create hosted Orbit tasks from `/ai <prompt>` and Slack messages
- stream task updates from Orbit into Slack threads
- resolve orphaned hosted-lane approvals with `Retry Lane` and `Cancel Task`
- inspect control-plane orphan policy with `/ai policy orphans ...`
- recover active Slack tasks and approval linkage from Orbit on startup

## Command Surface

### Slash commands

- `/ai <prompt>`: create a hosted Orbit task
- `/ai policy orphans`: show the default orphan policy and configured rule set
- `/ai policy orphans repo=<repo> source=<source> priority=<priority>`: preview the effective orphan policy for a target task shape

### Message handling

- non-bot messages handled by the connector create hosted Orbit tasks
- approval buttons in task threads resolve orphaned hosted-agent approvals

### Examples

```bash
/ai Fix the login bug in auth service
/ai policy orphans
/ai policy orphans repo=myorg/myapp source=api
/ai policy orphans repo=myorg/myapp source=slack priority=high
```

## Runtime Layout

The current connector implementation lives in `src/`:

- `src/index.ts`: process entrypoint
- `src/slack.ts`: Slack socket-mode interface, command handling, thread updates, approval actions
- `src/api-client.ts`: Orbit HTTP client for tasks, approvals, Slack connector callbacks, context updates, and orphan policy inspection
- `src/orbit-events.ts`: Orbit WebSocket event stream client with tracked-task scoped subscriptions
- `src/config.ts` and `src/env.ts`: validated environment configuration
- `src/log.ts`: connector logging
- `src/types.ts`: Slack and Orbit API types

## Required Environment

Copy `.env.example` to `.env` and configure:

```env
# Slack
SLACK_BOT_TOKEN=xoxb-your-bot-token
SLACK_APP_TOKEN=xapp-your-app-token
SLACK_SIGNING_SECRET=your-signing-secret

# Orbit server
ORBIT_API_URL=http://127.0.0.1:8788
ORBIT_API_TIMEOUT=30000

# App
NODE_ENV=development
LOG_LEVEL=info
PORT=3000

# Optional
GITHUB_TOKEN=
SENTRY_DSN=
MAX_CONCURRENT_TASKS=10
TASK_TIMEOUT=3600000
HEALTH_CHECK_INTERVAL=30000
```

## Development

Prerequisites:

- Node.js 18+
- a running Orbit server with hosted task APIs enabled
- Slack app configured for Socket Mode

Typical loop:

```bash
npm install
cp .env.example .env
npm run dev
```

Useful commands:

```bash
npm run sync:orbit-events
npm run build
npm test
npm run lint
npm run format
```

`npm run build` now checks that `src/generated/orbit-events.ts` matches the Rust event contract generator. If the file is stale, run `npm run sync:orbit-events` and rebuild.

## Slack App Setup

1. Create a Slack app and enable Socket Mode.
2. Configure a slash command for `/ai`.
3. Add bot token scopes:
   - `commands`
   - `chat:write`
   - `chat:write.public`
   - `users:read`
   - `channels:read`
4. Subscribe to the message events your workspace flow requires.

## Operator Workflows

### Inspect orphan policy

Use `/ai policy orphans` to inspect the default control-plane orphan policy and configured rule set.

Use selectors to preview the exact effective policy Orbit would apply:

```bash
/ai policy orphans repo=myorg/myapp source=api
/ai policy orphans repo=myorg/myapp source=slack priority=high
```

### Resolve orphan approvals

When Orbit emits `approval.requested` for an orphaned hosted agent, the connector renders `Retry Lane` and `Cancel Task` buttons in the Slack thread. Those actions resolve the server-side approval directly.

## Verification

```bash
npm run build
```

For an end-to-end operator runbook covering hosted task inspection, orphan recovery, approvals, and policy tuning, see `docs/hosted-task-operations.md`.
