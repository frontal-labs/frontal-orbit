# Orbit Slack Extension Development Guide

## Overview

This guide covers development setup, coding standards, testing practices, and contribution guidelines for the Orbit Slack extension.

## Development Environment

### Prerequisites

- **Node.js** 18+ 
- **Bun** package manager
- **Git** version control
- **VS Code** (recommended) with extensions:
  - TypeScript
  - ESLint
  - Prettier
  - Biome

### Setup

1. **Clone Repository**
   ```bash
   git clone <repository-url>
   cd extensions/orbit-slack
   ```

2. **Install Dependencies**
   ```bash
   bun install
   ```

3. **Configure Environment**
   ```bash
   cp .env.example .env
   # Edit .env with your development configuration
   ```

4. **Start Development Server**
   ```bash
   bun run dev
   ```

### Development Tools

#### VS Code Configuration

Create `.vscode/settings.json`:
```json
{
  "typescript.preferences.importModuleSpecifier": "relative",
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "biomejs.biome",
  "editor.codeActionsOnSave": {
    "quickfix.biome": "explicit",
    "source.organizeImports.biome": "explicit"
  },
  "files.exclude": {
    "**/node_modules": true,
    "**/dist": true,
    "**/.git": true
  }
}
```

#### Recommended Extensions

```json
{
  "recommendations": [
    "biomejs.biome",
    "ms-vscode.vscode-typescript-next",
    "bradlc.vscode-tailwindcss",
    "ms-vscode.vscode-json",
    "redhat.vscode-yaml"
  ]
}
```

## Project Structure

### Directory Layout

```
extensions/orbit-slack/
src/
  index.ts              # Application entry point
  slack.ts              # Slack client and command handling
  api-client.ts         # Orbit API client
  orbit-events.ts       # WebSocket event client
  config.ts             # Configuration management
  env.ts                # Environment validation
  log.ts                # Logging configuration
  types.ts              # Type definitions
  generated/
    orbit-events.ts      # Generated Orbit event types
scripts/
  check-orbit-events.mjs # Orbit events sync check
tests/
  unit/                 # Unit tests
  integration/          # Integration tests
  fixtures/             # Test fixtures
docs/                   # Documentation
```

### Core Components

#### Slack Client (`src/slack.ts`)

Handles Slack interactions:
- Socket Mode connection
- Command processing
- Message handling
- Interactive components

```typescript
export class SlackClient {
  private app: App;
  private apiClient: OrbitAPIClient;
  
  async start(): Promise<void>;
  async handleCommand(command: SlackCommand): Promise<void>;
  async handleMessage(message: SlackMessage): Promise<void>;
  async handleInteractiveAction(action: SlackAction): Promise<void>;
}
```

#### Orbit API Client (`src/api-client.ts`)

Manages Orbit server communication:
- Task CRUD operations
- Approval workflows
- Policy queries
- Callback handling

```typescript
export class OrbitAPIClient {
  private baseURL: string;
  private timeout: number;
  
  async createTask(request: CreateTaskRequest): Promise<Task>;
  async getTask(taskId: string): Promise<Task>;
  async updateTask(taskId: string, updates: UpdateTaskRequest): Promise<Task>;
  async resolveApproval(approvalId: string, action: ResolveApprovalRequest): Promise<Approval>;
}
```

#### Event Client (`src/orbit-events.ts`)

Handles real-time WebSocket events:
- Connection management
- Event subscriptions
- Message parsing
- Error handling

```typescript
export class OrbitEventClient {
  private ws: WebSocket;
  private subscriptions: Map<string, Set<string>>;
  
  async connect(): Promise<void>;
  async subscribe(taskId: string): Promise<void>;
  async unsubscribe(taskId: string): Promise<void>;
  private handleEvent(event: OrbitEvent): Promise<void>;
}
```

## Coding Standards

### TypeScript Guidelines

#### Type Definitions

```typescript
// Use interfaces for object shapes
interface Task {
  id: string;
  prompt: string;
  status: TaskStatus;
  createdAt: Date;
}

// Use union types for enums
type TaskStatus = 'pending' | 'running' | 'completed' | 'failed';

// Use generics for reusable types
interface ApiResponse<T> {
  data: T;
  success: boolean;
  error?: string;
}
```

#### Error Handling

```typescript
// Create custom error classes
export class OrbitAPIError extends Error {
  constructor(
    message: string,
    public code: string,
    public statusCode: number
  ) {
    super(message);
    this.name = 'OrbitAPIError';
  }
}

// Use Result pattern for operations
type Result<T, E = Error> = {
  success: true;
  data: T;
} | {
  success: false;
  error: E;
};

async function createTask(request: CreateTaskRequest): Promise<Result<Task>> {
  try {
    const task = await apiClient.createTask(request);
    return { success: true, data: task };
  } catch (error) {
    return { success: false, error: error as Error };
  }
}
```

#### Async Patterns

```typescript
// Use async/await consistently
async function handleCommand(command: SlackCommand): Promise<void> {
  const taskResult = await createTask({
    prompt: command.text,
    source: 'slack'
  });
  
  if (!taskResult.success) {
    throw new Error(`Failed to create task: ${taskResult.error.message}`);
  }
  
  await slackClient.postMessage({
    channel: command.channel_id,
    text: `Task created: ${taskResult.data.id}`
  });
}

// Use Promise.all for parallel operations
async function fetchMultipleTasks(taskIds: string[]): Promise<Task[]> {
  const tasks = await Promise.all(
    taskIds.map(id => apiClient.getTask(id))
  );
  return tasks;
}
```

### Code Organization

#### Module Structure

```typescript
// Export types first
export type { Task, TaskStatus, CreateTaskRequest };

// Export classes
export { OrbitAPIClient, SlackClient, OrbitEventClient };

// Export utilities
export { createTask, validateTaskRequest };

// Internal helpers (not exported)
function validateTaskRequest(request: CreateTaskRequest): boolean {
  return request.prompt.length > 0 && request.source.length > 0;
}
```

#### Dependency Injection

```typescript
// Use constructor injection
export class SlackService {
  constructor(
    private slackClient: SlackClient,
    private apiClient: OrbitAPIClient,
    private logger: Logger
  ) {}
  
  async processCommand(command: SlackCommand): Promise<void> {
    this.logger.info('Processing command', { commandId: command.id });
    
    try {
      await this.slackClient.acknowledgeCommand(command);
      const task = await this.apiClient.createTask({
        prompt: command.text,
        source: 'slack'
      });
      
      await this.slackClient.postMessage({
        channel: command.channel_id,
        text: `Task created: ${task.id}`
      });
    } catch (error) {
      this.logger.error('Command processing failed', { error, command });
      throw error;
    }
  }
}
```

### Testing Guidelines

#### Unit Tests

```typescript
// tests/unit/api-client.test.ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { OrbitAPIClient } from '../../src/api-client';

describe('OrbitAPIClient', () => {
  let client: OrbitAPIClient;
  let mockFetch: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockFetch = vi.fn();
    global.fetch = mockFetch;
    client = new OrbitAPIClient('http://test.com');
  });

  it('should create task successfully', async () => {
    const mockTask = { id: '123', prompt: 'test', status: 'pending' };
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => mockTask
    });

    const result = await client.createTask({ prompt: 'test', source: 'slack' });

    expect(result).toEqual(mockTask);
    expect(mockFetch).toHaveBeenCalledWith(
      'http://test.com/tasks',
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ prompt: 'test', source: 'slack' })
      })
    );
  });

  it('should handle API errors', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 400,
      json: async () => ({ error: 'Invalid request' })
    });

    await expect(client.createTask({ prompt: '', source: 'slack' }))
      .rejects.toThrow('Invalid request');
  });
});
```

#### Integration Tests

```typescript
// tests/integration/slack-integration.test.ts
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { SlackClient } from '../../src/slack';
import { MockOrbitServer } from '../fixtures/mock-orbit-server';

describe('Slack Integration', () => {
  let slackClient: SlackClient;
  let mockServer: MockOrbitServer;

  beforeAll(async () => {
    mockServer = new MockOrbitServer();
    await mockServer.start();
    
    slackClient = new SlackClient({
      botToken: 'xoxb-test',
      appToken: 'xapp-test',
      signingSecret: 'test-secret',
      orbitApiUrl: mockServer.url
    });
    
    await slackClient.start();
  });

  afterAll(async () => {
    await slackClient.stop();
    await mockServer.stop();
  });

  it('should process slash command and create task', async () => {
    const command = {
      command: '/ai',
      text: 'Fix the login bug',
      user_id: 'U123',
      channel_id: 'C123'
    };

    const response = await slackClient.handleCommand(command);

    expect(response.text).toContain('Task created');
    expect(mockServer.tasks).toHaveLength(1);
    expect(mockServer.tasks[0].prompt).toBe('Fix the login bug');
  });
});
```

#### Test Fixtures

```typescript
// tests/fixtures/mock-orbit-server.ts
export class MockOrbitServer {
  private server: any;
  public tasks: Task[] = [];
  public url: string;

  async start(): Promise<void> {
    this.server = new Express();
    this.setupRoutes();
    await new Promise(resolve => {
      this.server.listen(0, resolve);
    });
    this.url = `http://localhost:${this.server.address().port}`;
  }

  private setupRoutes(): void {
    this.server.post('/tasks', (req, res) => {
      const task = {
        id: generateId(),
        ...req.body,
        status: 'pending',
        createdAt: new Date()
      };
      this.tasks.push(task);
      res.json(task);
    });

    this.server.get('/tasks/:id', (req, res) => {
      const task = this.tasks.find(t => t.id === req.params.id);
      if (!task) return res.status(404).json({ error: 'Not found' });
      res.json(task);
    });
  }
}
```

#### Test Utilities

```typescript
// tests/utils/test-helpers.ts
export function createMockSlackCommand(overrides: Partial<SlackCommand> = {}): SlackCommand {
  return {
    command: '/ai',
    text: 'Test command',
    user_id: 'U123',
    channel_id: 'C123',
    team_id: 'T123',
    ...overrides
  };
}

export function createMockTask(overrides: Partial<Task> = {}): Task {
  return {
    id: generateId(),
    prompt: 'Test task',
    source: 'slack',
    status: 'pending',
    createdAt: new Date(),
    ...overrides
  };
}

export async function waitForEvent<T>(
  emitter: EventEmitter,
  event: string,
  timeout = 5000
): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new Error(`Event ${event} not received within ${timeout}ms`));
    }, timeout);

    emitter.once(event, (data: T) => {
      clearTimeout(timer);
      resolve(data);
    });
  });
}
```

## Development Workflow

### Git Workflow

#### Branch Naming

```bash
# Feature branches
feature/task-creation-improvements
feature/approval-workflow-enhancements

# Bugfix branches
bugfix/websocket-connection-timeout
bugfix/memory-leak-in-task-processing

# Hotfix branches
hotfix/critical-security-patch
hotfix/broken-slack-integration
```

#### Commit Messages

Follow Conventional Commits:

```bash
# Features
feat: add support for task priority levels
feat: implement approval workflow buttons

# Bug fixes
fix: resolve WebSocket connection timeout issue
fix: handle missing environment variables gracefully

# Documentation
docs: update API documentation
docs: add troubleshooting guide

# Refactoring
refactor: extract Slack client into separate module
refactor: improve error handling patterns

# Tests
test: add integration tests for approval workflows
test: increase test coverage for API client

# Chore
chore: update dependencies
chore: configure CI/CD pipeline
```

### Code Review Process

#### Pull Request Template

```markdown
## Description
Brief description of changes and motivation.

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed
- [ ] Test coverage maintained/improved

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Environment variables documented
- [ ] Error handling implemented
- [ ] Logging added where appropriate

## Screenshots (if applicable)
Add screenshots for UI changes.

## Additional Notes
Any additional context or considerations.
```

#### Review Guidelines

1. **Functionality**: Does the code work as intended?
2. **Testing**: Are tests comprehensive and passing?
3. **Documentation**: Is documentation updated and accurate?
4. **Style**: Does code follow project standards?
5. **Performance**: Are there performance implications?
6. **Security**: Are security considerations addressed?

### Local Development

#### Environment Setup

```bash
# Start development environment
bun run dev

# Run tests in watch mode
bun run test:watch

# Run linting
bun run lint

# Format code
bun run format
```

#### Debugging

```bash
# Enable debug logging
LOG_LEVEL=debug bun run dev

# Enable Node.js debugging
NODE_OPTIONS="--inspect-brk" bun run dev

# Profile memory usage
NODE_OPTIONS="--prof" bun run dev
```

#### Hot Reloading

The development server supports hot reloading for TypeScript files:

```typescript
// Use nodemon configuration
{
  "watch": ["src"],
  "ext": "ts",
  "exec": "bun run dev",
  "ignore": ["src/**/*.test.ts"]
}
```

## Performance Guidelines

### Memory Management

```typescript
// Clean up event listeners
export class EventManager {
  private listeners: Map<string, Function[]> = new Map();

  on(event: string, listener: Function): void {
    const listeners = this.listeners.get(event) || [];
    listeners.push(listener);
    this.listeners.set(event, listeners);
  }

  off(event: string, listener: Function): void {
    const listeners = this.listeners.get(event);
    if (listeners) {
      const index = listeners.indexOf(listener);
      if (index > -1) {
        listeners.splice(index, 1);
      }
    }
  }

  cleanup(): void {
    this.listeners.clear();
  }
}
```

### Connection Pooling

```typescript
// Reuse HTTP connections
import { Agent } from 'undici';

const agent = new Agent({
  connections: 10,
  keepAliveTimeout: 60000
});

export class HttpClient {
  constructor(private agent: Agent) {}

  async request(url: string, options: RequestOptions): Promise<Response> {
    return fetch(url, { ...options, dispatcher: this.agent });
  }
}
```

### Caching Strategy

```typescript
// Implement caching for frequently accessed data
export class CacheManager {
  private cache: Map<string, { data: any; expiry: number }> = new Map();

  set(key: string, data: any, ttl: number): void {
    this.cache.set(key, {
      data,
      expiry: Date.now() + ttl
    });
  }

  get(key: string): any | null {
    const item = this.cache.get(key);
    if (!item) return null;

    if (Date.now() > item.expiry) {
      this.cache.delete(key);
      return null;
    }

    return item.data;
  }
}
```

## Security Guidelines

### Input Validation

```typescript
// Validate all external inputs
import z from 'zod';

const TaskRequestSchema = z.object({
  prompt: z.string().min(1).max(1000),
  source: z.enum(['slack', 'api', 'web']),
  priority: z.enum(['low', 'medium', 'high']).optional()
});

export function validateTaskRequest(request: unknown): CreateTaskRequest {
  return TaskRequestSchema.parse(request);
}
```

### Secret Management

```typescript
// Never log sensitive data
export function sanitizeLogData(data: any): any {
  const sensitiveFields = ['token', 'password', 'secret', 'key'];
  const sanitized = { ...data };

  for (const field of sensitiveFields) {
    if (field in sanitized) {
      sanitized[field] = '[REDACTED]';
    }
  }

  return sanitized;
}
```

### Request Verification

```typescript
// Verify Slack request signatures
export function verifySlackRequest(
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

## Monitoring and Observability

### Logging

```typescript
// Structured logging
export class Logger {
  constructor(private context: Record<string, any>) {}

  info(message: string, data?: Record<string, any>): void {
    console.log(JSON.stringify({
      level: 'info',
      message,
      timestamp: new Date().toISOString(),
      ...this.context,
      ...data
    }));
  }

  error(message: string, error?: Error, data?: Record<string, any>): void {
    console.error(JSON.stringify({
      level: 'error',
      message,
      timestamp: new Date().toISOString(),
      ...this.context,
      ...data,
      error: error ? {
        name: error.name,
        message: error.message,
        stack: error.stack
      } : undefined
    }));
  }
}
```

### Metrics

```typescript
// Custom metrics
export class Metrics {
  private counters: Map<string, number> = new Map();
  private histograms: Map<string, number[]> = new Map();

  increment(name: string, value = 1): void {
    this.counters.set(name, (this.counters.get(name) || 0) + value);
  }

  histogram(name: string, value: number): void {
    const values = this.histograms.get(name) || [];
    values.push(value);
    this.histograms.set(name, values);
  }

  getMetrics(): Record<string, any> {
    return {
      counters: Object.fromEntries(this.counters),
      histograms: Object.fromEntries(
        Array.from(this.histograms.entries()).map(([name, values]) => [
          name,
          {
            count: values.length,
            sum: values.reduce((a, b) => a + b, 0),
            avg: values.reduce((a, b) => a + b, 0) / values.length
          }
        ])
      )
    };
  }
}
```

## Release Process

### Version Management

Use semantic versioning:

```bash
# Patch version (bug fixes)
bun version patch

# Minor version (new features)
bun version minor

# Major version (breaking changes)
bun version major
```

### Release Checklist

1. **Testing**
   - [ ] All tests pass
   - [ ] Integration tests validated
   - [ ] Manual testing completed

2. **Documentation**
   - [ ] API documentation updated
   - [ ] README updated
   - [ ] Changelog updated

3. **Security**
   - [ ] Dependencies audited
   - [ ] Security review completed
   - [ ] Secrets verified

4. **Performance**
   - [ ] Performance tests run
   - [ ] Memory usage verified
   - [ ] Load testing completed

5. **Deployment**
   - [ ] Docker image built
   - [ ] Configuration validated
   - [ ] Rollback plan prepared

### Deployment

```bash
# Build for production
bun run build

# Create Docker image
docker build -t orbit-slack:v0.1.0 .

# Tag and push
docker tag orbit-slack:v0.1.0 your-registry/orbit-slack:v0.1.0
docker push your-registry/orbit-slack:v0.1.0

# Deploy
kubectl set image deployment/orbit-slack \
  orbit-slack=your-registry/orbit-slack:v0.1.0
```

This development guide provides comprehensive guidelines for contributing to the Orbit Slack extension.
