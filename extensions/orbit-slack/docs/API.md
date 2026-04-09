# Orbit Slack Extension API Reference

## Overview

The Orbit Slack extension provides both HTTP endpoints for Slack integration and client APIs for interacting with the Orbit server.

## HTTP Endpoints

### Slack Event Callbacks

#### POST /slack/events

Handles Slack event callbacks for message events and app mentions.

**Request Body:**
```json
{
  "type": "url_verification",
  "challenge": "challenge_string",
  "token": "verification_token"
}
```

**Response:**
```json
{
  "challenge": "challenge_string"
}
```

#### POST /slack/commands

Processes slash commands from Slack.

**Request Body:**
```json
{
  "command": "/ai",
  "text": "Fix the login bug",
  "response_url": "https://hooks.slack.com/...",
  "user_id": "U1234567890",
  "channel_id": "C1234567890"
}
```

**Response:**
```json
{
  "response_type": "in_channel",
  "text": "Task created successfully"
}
```

#### POST /slack/interactive

Handles interactive component responses (button clicks, etc.).

**Request Body:**
```json
{
  "type": "block_actions",
  "actions": [
    {
      "action_id": "retry_lane",
      "block_id": "approval_block",
      "value": "task_123"
    }
  ],
  "user": {
    "id": "U1234567890"
  },
  "channel": {
    "id": "C1234567890"
  }
}
```

### Health Check

#### GET /health

Returns the health status of the extension.

**Response:**
```json
{
  "status": "ok",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "0.1.0"
}
```

## Orbit API Client

### Task Management

#### createTask

Creates a new hosted task in Orbit.

```typescript
interface CreateTaskRequest {
  prompt: string;
  source: string;
  priority?: 'low' | 'medium' | 'high';
  metadata?: Record<string, any>;
}

interface Task {
  id: string;
  prompt: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  createdAt: string;
  updatedAt: string;
}
```

**Example:**
```typescript
const task = await apiClient.createTask({
  prompt: "Fix the login bug in auth service",
  source: "slack",
  priority: "medium"
});
```

#### getTask

Retrieves a task by ID.

```typescript
const task = await apiClient.getTask(taskId);
```

#### updateTask

Updates task metadata or status.

```typescript
interface UpdateTaskRequest {
  status?: TaskStatus;
  metadata?: Record<string, any>;
}

const updatedTask = await apiClient.updateTask(taskId, updates);
```

### Approval Management

#### getApproval

Retrieves approval information for a task.

```typescript
interface Approval {
  id: string;
  taskId: string;
  status: 'pending' | 'approved' | 'rejected';
  requestedAt: string;
  resolvedAt?: string;
}

const approval = await apiClient.getApproval(approvalId);
```

#### resolveApproval

Resolves an approval (approve or reject).

```typescript
interface ResolveApprovalRequest {
  action: 'approve' | 'reject';
  reason?: string;
}

const resolvedApproval = await apiClient.resolveApproval(
  approvalId, 
  { action: 'approve' }
);
```

### Policy Management

#### getOrphanPolicy

Retrieves the current orphan policy configuration.

```typescript
interface OrphanPolicy {
  defaultPolicy: string;
  rules: PolicyRule[];
}

interface PolicyRule {
  condition: string;
  action: string;
  priority?: number;
}

const policy = await apiClient.getOrphanPolicy();
```

#### previewOrphanPolicy

Previews the effective policy for a specific task configuration.

```typescript
interface PolicyPreviewRequest {
  repo?: string;
  source?: string;
  priority?: 'low' | 'medium' | 'high';
}

interface PolicyPreview {
  effectivePolicy: string;
  matchedRule?: PolicyRule;
}

const preview = await apiClient.previewOrphanPolicy({
  repo: "myorg/myapp",
  source: "slack",
  priority: "high"
});
```

## WebSocket Events

### Event Types

#### task.created

Fired when a new task is created.

```typescript
interface TaskCreatedEvent {
  type: "task.created";
  data: {
    task: Task;
    timestamp: string;
  };
}
```

#### task.updated

Fired when a task status changes.

```typescript
interface TaskUpdatedEvent {
  type: "task.updated";
  data: {
    taskId: string;
    previousStatus: TaskStatus;
    currentStatus: TaskStatus;
    timestamp: string;
  };
}
```

#### task.completed

Fired when a task completes successfully.

```typescript
interface TaskCompletedEvent {
  type: "task.completed";
  data: {
    task: Task;
    result: any;
    timestamp: string;
  };
}
```

#### approval.requested

Fired when an approval is requested.

```typescript
interface ApprovalRequestedEvent {
  type: "approval.requested";
  data: {
    approval: Approval;
    task: Task;
    timestamp: string;
  };
}
```

#### approval.resolved

Fired when an approval is resolved.

```typescript
interface ApprovalResolvedEvent {
  type: "approval.resolved";
  data: {
    approval: Approval;
    task: Task;
    timestamp: string;
  };
}
```

### WebSocket Client

```typescript
interface OrbitEventClient {
  connect(): Promise<void>;
  subscribe(taskId: string): void;
  unsubscribe(taskId: string): void;
  on(event: string, callback: (data: any) => void): void;
  disconnect(): void;
}
```

**Example:**
```typescript
const client = new OrbitEventClient(apiClient);

await client.connect();

client.subscribe(taskId);

client.on('task.updated', (event) => {
  console.log('Task updated:', event.data);
});
```

## Configuration API

### Environment Validation

The extension validates environment variables on startup:

```typescript
interface Config {
  slack: {
    botToken: string;
    appToken: string;
    signingSecret: string;
  };
  orbit: {
    apiUrl: string;
    timeout: number;
  };
  app: {
    nodeEnv: string;
    logLevel: string;
    port: number;
  };
  github?: {
    token: string;
  };
  sentry?: {
    dsn: string;
  };
  performance: {
    maxConcurrentTasks: number;
    taskTimeout: number;
    healthCheckInterval: number;
  };
}
```

### Error Handling

The extension uses structured error handling:

```typescript
interface APIError {
  code: string;
  message: string;
  details?: any;
  timestamp: string;
}

// Example error response
{
  "code": "TASK_NOT_FOUND",
  "message": "Task with ID '123' not found",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

## Rate Limiting

### Slack API Limits

- Message posting: 1 message per channel per second
- Interactive responses: Must respond within 3 seconds
- Event acknowledgments: Must respond within 3 seconds

### Orbit API Limits

- Task creation: 10 requests per minute
- Task updates: 60 requests per minute
- Policy queries: 30 requests per minute

## Authentication

### Slack Request Verification

All incoming Slack requests are verified using the signing secret:

```typescript
function verifySlackRequest(
  signature: string,
  timestamp: string,
  body: string,
  signingSecret: string
): boolean {
  const expectedSignature = 'sha256=' + crypto
    .createHmac('sha256', signingSecret)
    .update(timestamp + body)
    .digest('hex');
  
  return crypto.timingSafeEqual(
    Buffer.from(signature),
    Buffer.from(expectedSignature)
  );
}
```

### Orbit API Authentication

The extension uses API tokens for Orbit API authentication:

```typescript
const headers = {
  'Authorization': `Bearer ${apiToken}`,
  'Content-Type': 'application/json'
};
```

## Data Models

### Task Models

```typescript
interface Task {
  id: string;
  prompt: string;
  source: string;
  priority: TaskPriority;
  status: TaskStatus;
  metadata: TaskMetadata;
  createdAt: string;
  updatedAt: string;
  completedAt?: string;
}

interface TaskMetadata {
  slackChannelId?: string;
  slackMessageTs?: string;
  slackUserId?: string;
  repo?: string;
  branch?: string;
  [key: string]: any;
}

type TaskPriority = 'low' | 'medium' | 'high';
type TaskStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
```

### Slack Models

```typescript
interface SlackMessage {
  channel: string;
  ts: string;
  text: string;
  user: string;
  thread_ts?: string;
  blocks?: SlackBlock[];
}

interface SlackBlock {
  type: string;
  block_id?: string;
  text?: SlackText;
  elements?: SlackElement[];
}

interface SlackText {
  type: 'plain_text' | 'mrkdwn';
  text: string;
  emoji?: boolean;
}
```

## Testing Utilities

### Mock Clients

The extension provides mock clients for testing:

```typescript
class MockOrbitAPIClient implements OrbitAPIClient {
  async createTask(request: CreateTaskRequest): Promise<Task> {
    // Mock implementation
  }
  
  async getTask(taskId: string): Promise<Task> {
    // Mock implementation
  }
}
```

### Test Fixtures

```typescript
interface TestFixtures {
  sampleTask: Task;
  sampleApproval: Approval;
  sampleSlackEvent: SlackEvent;
}
```

## Migration Guide

### From v0.0.x to v0.1.0

- Updated TypeScript definitions
- Added WebSocket event streaming
- Improved error handling
- Enhanced configuration validation

### Breaking Changes

- `createTask` now requires `source` parameter
- Event types have been renamed for consistency
- Configuration structure has changed
