# TypeScript SDK API Reference

## Module: `@frontal-labs/orbit-sdk`

### `Orbit`

#### `new Orbit(config?: OrbitConfig)`
Create a new Orbit client.

**Parameters:**
- `config?.command` - Path to `orbit` binary (default: `"orbit"`)
- `config?.baseUrl` - Override API base URL (maps to `--config frontal_base_url`)
- `config?.env` - Environment variables to pass to CLI process
- `config?.config` - Global `--config` key/value pairs

#### `orbit.startThread(options?: ThreadOptions): Thread`
Start a new conversation thread.

#### `orbit.resumeThread(threadId: string, options?: ThreadOptions): Thread`
Resume an existing thread by session ID.

---

### `Thread`

#### `thread.id: string`
Readonly session ID.

#### `thread.run(input: ThreadInput, options?: ThreadRunOptions): Promise<TurnResult>`
Run a turn and buffer all events until completion.

#### `thread.runStreamed(input: ThreadInput, options?: ThreadRunOptions): Promise<StreamedTurn>`
Run a turn and return an async generator of events for real-time processing.

---

### Types

#### `OrbitConfig`

```typescript
interface OrbitConfig {
  command?: string;                    // default: "orbit"
  baseUrl?: string;                    // maps to frontal_base_url
  env?: Record<string, string>;        // additional env vars
  config?: Record<string, unknown>;    // global --config overrides
}
```

#### `ThreadOptions`

```typescript
interface ThreadOptions {
  workingDirectory?: string;           // working directory for CLI
  skipGitRepoCheck?: boolean;          // allow non-git directories
  config?: Record<string, unknown>;    // per-thread --config overrides
}
```

#### `ThreadRunOptions`

```typescript
interface ThreadRunOptions {
  outputSchema?: Record<string, unknown>;  // JSON Schema for structured output
  config?: Record<string, unknown>;        // per-turn --config overrides
}
```

#### `ThreadInput`

```typescript
type ThreadInput = 
  | string                              // simple text
  | ThreadInputEntry                    // single entry
  | ThreadInputEntry[];                 // multiple entries

interface ThreadInputEntry {
  type: 'text' | 'local_image';
  text?: string;                        // required when type === 'text'
  path?: string;                        // required when type === 'local_image'
}
```

#### `TurnResult`

```typescript
interface TurnResult {
  finalResponse: string;
  items: Item[];
  usage?: Usage;
}
```

#### `StreamedTurn`

```typescript
interface StreamedTurn {
  events: AsyncGenerator<StreamEvent, void, unknown>;
}
```

#### `StreamEvent`

```typescript
type StreamEvent = 
  | { type: 'item_completed'; item: Item }
  | { type: 'turn_completed'; result: TurnResult }
  | { type: 'error'; error: { message: string; code?: string } };
```

#### `Item`

```typescript
interface Item {
  id: string;
  type: 'message' | 'tool_call' | 'tool_result' | 'reasoning';
  content?: string;
  toolCall?: ToolCall;
  toolResult?: ToolResult;
}

interface ToolCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
}

interface ToolResult {
  toolCallId: string;
  content: string;
  isError?: boolean;
}
```

#### `Usage`

```typescript
interface Usage {
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
}
```

#### `OrbitError`

```typescript
class OrbitError extends Error {
  code: OrbitErrorCode;
  constructor(message: string, code: OrbitErrorCode);
}

type OrbitErrorCode = 
  | 'CLI_NOT_FOUND'
  | 'PROCESS_EXITED'
  | 'INVALID_EVENT'
  | 'THREAD_NOT_FOUND'
  | 'IO_ERROR'
  | 'JSON_PARSE_ERROR';
```

---

## Usage Examples

### Basic Usage

```typescript
import { Orbit } from '@frontal-labs/orbit-sdk';

const orbit = new Orbit();
const thread = orbit.startThread();

const turn = await thread.run('Explain async/await in TypeScript');
console.log(turn.finalResponse);
```

### Streaming

```typescript
const { events } = await thread.runStreamed('Refactor this code');

for await (const event of events) {
  if (event.type === 'item_completed') {
    console.log(`[${event.item.type}] ${event.item.content ?? ''}`);
  } else if (event.type === 'turn_completed') {
    console.log('Done:', event.result.usage);
  }
}
```

### Structured Output

```typescript
const schema = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    issues: { type: 'array', items: { type: 'string' } },
    severity: { type: 'string', enum: ['low', 'medium', 'high', 'critical'] }
  },
  required: ['summary', 'severity'],
  additionalProperties: false
} as const;

const turn = await thread.run('Analyze security', { outputSchema: schema });
const result = JSON.parse(turn.finalResponse); // Guaranteed valid
```

### Image Input

```typescript
const turn = await thread.run([
  { type: 'text', text: 'Describe this UI' },
  { type: 'local_image', path: './screenshot.png' }
]);
```

### Resume Thread

```typescript
const savedId = process.env.CODEX_THREAD_ID!;
const thread = orbit.resumeThread(savedId);
await thread.run('Continue the fix');
```

### Config Overrides

```typescript
// Global
const orbit = new Orbit({
  config: {
    show_raw_agent_reasoning: true,
    sandbox_workspace_write: { network_access: true }
  }
});

// Per-thread
const thread = orbit.startThread({
  config: { model: 'gpt-4' }
});

// Per-turn
const turn = await thread.run('Be precise', {
  config: { temperature: 0.1 }
});
```

### Working Directory

```typescript
const thread = orbit.startThread({
  workingDirectory: '/path/to/project',
  skipGitRepoCheck: true
});
```

### Custom CLI Path / Env

```typescript
const orbit = new Orbit({
  command: '/custom/orbit',
  baseUrl: 'https://api.example.com',
  env: { CUSTOM_VAR: 'value' }
});
```

### Error Handling

```typescript
import { OrbitError, OrbitErrorCode } from '@frontal-labs/orbit-sdk';

try {
  await thread.run('test');
} catch (error) {
  if (error instanceof OrbitError) {
    switch (error.code) {
      case 'CLI_NOT_FOUND':
        console.error('Install @frontal-labs/orbit');
        break;
      case 'PROCESS_EXITED':
        console.error('CLI crashed');
        break;
      case 'THREAD_NOT_FOUND':
        console.error('Session expired');
        break;
    }
  }
}
```

---

## Zod Schema Integration

```typescript
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';

const schema = z.object({
  summary: z.string(),
  status: z.enum(['ok', 'action_required'])
});

const turn = await thread.run('Summarize', {
  outputSchema: zodToJsonSchema(schema, { target: 'frontal' })
});

const result = schema.parse(JSON.parse(turn.finalResponse));
```

---

## Package Exports

```typescript
// Main entry
import { Orbit, Thread } from '@frontal-labs/orbit-sdk';

// Types
import type {
  OrbitConfig,
  ThreadOptions,
  ThreadRunOptions,
  ThreadInput,
  TurnResult,
  StreamEvent,
  Item,
  Usage,
  OrbitError,
  OrbitErrorCode
} from '@frontal-labs/orbit-sdk';

// Errors
import { OrbitError } from '@frontal-labs/orbit-sdk/errors';
```

---

## Requirements

- Node.js 18+
- `@frontal-labs/orbit` CLI installed and on PATH (or custom `command`)

## Version Compatibility

| SDK Version | CLI Version |
|-------------|-------------|
| 0.1.x       | 0.1.x       |