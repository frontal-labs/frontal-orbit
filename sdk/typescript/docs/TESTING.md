# TypeScript SDK Testing Guide

## Test Architecture

Tests are fully hermetic - they do **not** require:
- Network access
- API keys (`CODEX_API_KEY`)
- Real `orbit` CLI binary

Instead, tests use a mock CLI (`tests/mock-orbit.ts`) that emits
predefined JSONL events over stdout.

## Running Tests

```bash
# Run all tests
npm test

# Run with coverage
npm run test:coverage

# Run specific test file
npm test -- tests/thread.test.ts

# Watch mode
npm run test:watch
```

## Test Structure

```
tests/
├── mock-orbit.ts           # Mock CLI - emits JSONL events
├── test-utils.ts           # Shared test utilities
├── thread.test.ts          # Basic thread run tests
├── streaming.test.ts       # Stream event tests
├── structured-output.test.ts # JSON Schema / structured output tests
├── resume.test.ts          # Thread resume tests
├── config.test.ts          # Config override tests
├── images.test.ts          # Image input tests
└── errors.test.ts          # Error handling tests
```

## Mock CLI Protocol

The mock CLI (`tests/mock-orbit.ts`) reads stdin for the request payload
and writes JSONL events to stdout.

### Request Format

```typescript
interface MockRequest {
  input: string;
  session_id?: string;
  config?: Record<string, unknown>;
  output_schema?: Record<string, unknown>;
}
```

### Response Format (JSONL)

```jsonl
{"type":"item_completed","item":{"id":"msg-1","type":"message","content":"Hello!"}}
{"type":"turn_completed","result":{"final_response":"Hello!","items":[],"usage":{}}}
```

### Adding New Test Scenarios

Edit `tests/mock-orbit.ts`:

```typescript
const SCENARIOS: Record<string, string[]> = {
  // Existing scenarios...
  
  my_new_scenario: [
    '{"type":"item_completed","item":{"id":"msg-1","type":"message","content":"Custom response"}}',
    '{"type":"turn_completed","result":{"final_response":"Custom response","items":[],"usage":{}}}'
  ],
};
```

Then use in tests:

```typescript
import { createMockOrbit } from './test-utils';

test('my new scenario', async () => {
  const orbit = createMockOrbit({ scenario: 'my_new_scenario' });
  const thread = orbit.startThread();
  const result = await thread.run('test input');
  
  expect(result.finalResponse).toBe('Custom response');
});
```

## Test Utilities

### `createMockOrbit(options)`

```typescript
const orbit = createMockOrbit({
  scenario: 'basic',           // scenario name from mock-orbit.ts
  env: { CUSTOM_VAR: 'value' }, // extra env vars
  cwd: '/custom/dir'           // working directory
});
```

### `assertEventSequence(events, ...expectedTypes)`

```typescript
import { assertEventSequence } from './test-utils';

const { events } = await thread.runStreamed('input');
assertEventSequence(events, 'item_completed', 'turn_completed');
```

### `collectEvents(events)`

```typescript
import { collectEvents } from './test-utils';

const { events } = await thread.runStreamed('input');
const allEvents = await collectEvents(events);

expect(allEvents.filter(e => e.type === 'item_completed')).toHaveLength(3);
```

## Writing New Tests

### Basic Pattern

```typescript
import { createMockOrbit } from './test-utils';
import { OrbitError } from '../src/errors';

test('descriptive test name', async () => {
  // Arrange
  const orbit = createMockOrbit({ scenario: 'my_scenario' });
  const thread = orbit.startThread();
  
  // Act
  const result = await thread.run('test input');
  
  // Assert
  expect(result.finalResponse).toBe('expected output');
  expect(result.items).toHaveLength(1);
});
```

### Streaming Test

```typescript
test('streams tool calls in real-time', async () => {
  const orbit = createMockOrbit({ scenario: 'streaming_tool_calls' });
  const thread = orbit.startThread();
  
  const { events } = await thread.runStreamed('run tests');
  
  const toolCalls: Item[] = [];
  for await (const event of events) {
    if (event.type === 'item_completed' && event.item.type === 'tool_call') {
      toolCalls.push(event.item);
    }
  }
  
  expect(toolCalls.length).toBeGreaterThan(0);
});
```

### Error Test

```typescript
test('throws OrbitError when CLI not found', async () => {
  const orbit = new Orbit({ command: '/nonexistent/orbit' });
  const thread = orbit.startThread();
  
  await expect(thread.run('test')).rejects.toThrow(OrbitError);
  await expect(thread.run('test')).rejects.toMatchObject({
    code: 'CLI_NOT_FOUND'
  });
});
```

### Config Override Test

```typescript
test('passes config overrides to CLI', async () => {
  const orbit = createMockOrbit({ scenario: 'echo_config' });
  const thread = orbit.startThread();
  
  await thread.run('test', {
    config: { temperature: 0.5, model: 'gpt-4' }
  });
  
  // mock-orbit echoes received config in response
  // assert config was passed correctly
});
```

## Mock Scenario Reference

| Scenario | Description |
|----------|-------------|
| `basic` | Simple text response |
| `streaming` | Multiple item_completed events |
| `streaming_tool_calls` | Tool call events in stream |
| `structured_output` | Valid JSON matching schema |
| `resume_thread` | Session resume flow |
| `image_input` | Image input handling |
| `config_echo` | Echoes received config |
| `error_cli_exit` | CLI process exits with error |
| `error_invalid_json` | Malformed JSONL from CLI |

## Continuous Integration

Tests run in GitHub Actions on every PR:

```yaml
# .github/workflows/typescript-sdk.yml
- name: Run tests
  run: npm test
  
- name: Check coverage
  run: npm run test:coverage
```

## Coverage Goals

- Statements: >90%
- Branches: >85%
- Functions: >90%
- Lines: >90%

Run `npm run test:coverage` and open `coverage/lcov-report/index.html` for details.