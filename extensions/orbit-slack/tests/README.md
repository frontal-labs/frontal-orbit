# Orbit Slack Bot Test Suite

This directory contains comprehensive tests for the Orbit Slack bot extension using Vitest.

## Test Structure

```
tests/
|-- README.md                 # This file
|-- setup.ts                  # Global test setup and mocks
|-- utils/
|   |-- env.test.ts           # Environment variable validation tests
|   |-- validators.test.ts     # Input validation tests
|   |-- logger.test.ts         # Logging utility tests
|   `-- config.test.ts        # Configuration tests
|-- types/
|   |-- slack-types.test.ts    # Slack type definitions tests
|   `-- orbit-types.test.ts    # Orbit type definitions tests
|-- bot/
|   `-- slack-bot-service.test.ts # Main bot service tests
|-- services/
|   |-- orbit-api-client.test.ts   # Orbit API client tests
|   |-- database-service.test.ts   # Database service tests
|   |-- redis-service.test.ts       # Redis service tests
|   |-- task-manager.test.ts        # Task manager tests
|   `-- conversation-manager.test.ts # Conversation manager tests
`-- integration/
    `-- slack-workflow.test.ts     # End-to-end workflow tests
```

## Test Categories

### Unit Tests
- **Utils Tests**: Test individual utility functions for validation, configuration, and logging
- **Types Tests**: Verify type definitions and runtime type safety
- **Service Tests**: Test individual service classes in isolation

### Integration Tests
- **Workflow Tests**: Test complete user workflows from start to finish
- **API Integration**: Test integration with external APIs (Orbit, Slack)
- **Multi-user Scenarios**: Test concurrent user interactions

## Running Tests

### Basic Commands
```bash
# Run all tests
npm test

# Run tests in watch mode
npm run test:watch

# Run tests with coverage
npm run test:coverage

# Run tests with UI interface
npm run test:ui

# Run tests once (CI mode)
npm run ci
```

### Individual Test Files
```bash
# Run specific test file
npx vitest tests/utils/env.test.ts

# Run tests in a directory
npx vitest tests/utils/

# Run tests matching a pattern
npx vitest --grep "environment variables"
```

## Test Configuration

### Vitest Configuration (`vitest.config.ts`)
- **Environment**: jsdom for DOM testing
- **Coverage**: 80% threshold for all metrics
- **Timeout**: 10 seconds for tests and hooks
- **Setup**: Global setup file for mocks and environment
- **Path Aliases**: `@/` mapped to `src/` directory

### Global Setup (`tests/setup.ts`)
- **Environment Variables**: Mock all required environment variables
- **MSW Server**: Mock HTTP requests to Orbit API and Slack API
- **Console Mocks**: Reduce test noise by mocking console methods
- **Mock Factories**: Helper functions for creating test data
- **Service Mocks**: Pre-configured mocks for all service dependencies

## Test Data and Mocks

### Mock Factories
```typescript
// Create mock Slack command
const command = createMockSlackCommand({
  text: 'Test task description',
  user_id: 'U123456',
});

// Create mock task
const task = createMockSlackTask({
  status: 'running',
  slack_task_id: 'task-123',
});

// Create mock user preferences
const preferences = createMockUserPreferences({
  default_model: 'claude-3-sonnet',
});
```

### Service Mocks
```typescript
// Mock Orbit API client
mockOrbitApiClient.submitPrompt.mockResolvedValue({
  ok: true,
  stdout: 'Task completed',
});

// Mock Database service
mockDatabaseService.createSlackTask.mockResolvedValue(mockTask);

// Mock Redis service
mockRedisService.set.mockResolvedValue('OK');
```

## Testing Best Practices

### Test Structure
```typescript
describe('Feature Being Tested', () => {
  beforeEach(() => {
    // Setup for each test
    vi.clearAllMocks();
  });

  describe('Specific Scenario', () => {
    it('should behave as expected', async () => {
      // Arrange - setup test data and mocks
      // Act - execute the code being tested
      // Assert - verify the expected behavior
    });
  });
});
```

### Mock Management
- **Clear mocks** in `beforeEach` to ensure test isolation
- **Use specific mock return values** for predictable test results
- **Mock external dependencies** to avoid network calls
- **Restore mocks** after tests to prevent side effects

### Assertions
- **Use specific assertions** (`expect(value).toBe(expected)`)
- **Test error cases** (`expect(fn).toThrow()`)
- **Verify mock calls** (`expect(mockFn).toHaveBeenCalledWith(args)`)
- **Check types** where relevant (`expect(typeof value).toBe('string')`)

## Coverage Requirements

The test suite maintains **80% coverage** across all metrics:
- **Statements**: 80%
- **Branches**: 80%
- **Functions**: 80%
- **Lines**: 80%

Coverage reports are generated in `coverage/` directory:
- `coverage/index.html` - Interactive HTML report
- `coverage/coverage.json` - Machine-readable JSON report
- `coverage/lcov.info` - LCOV format for CI integration

## Test Scenarios Covered

### Command Handling
- **Create Command**: Valid/invalid commands, parsing flags, error handling
- **Status Command**: Specific tasks, recent tasks, not found cases
- **List Command**: With/without tasks, pagination
- **Cancel Command**: Valid/invalid task IDs, permission checks
- **Help Command**: Help content display

### Message Handling
- **Hello Messages**: Greeting responses
- **Direct Messages**: Task creation from natural language
- **App Mentions**: Task creation from mentions
- **Error Handling**: Invalid input, service failures

### Interaction Handling
- **Button Actions**: Task cancellation, pause/resume (when implemented)
- **Error Responses**: Invalid interactions, permission issues

### Service Integration
- **Orbit API**: Prompt submission, status checks, health checks
- **Database**: Task CRUD operations, user management
- **Redis**: Caching, session management, rate limiting
- **Slack API**: Authentication, message posting

### Error Scenarios
- **Validation Errors**: Invalid input, missing required fields
- **Service Errors**: Network failures, database errors
- **Permission Errors**: Unauthorized actions, rate limits
- **Timeout Errors**: Long-running operations

### Multi-User Scenarios
- **Concurrent Tasks**: Multiple users creating tasks simultaneously
- **Isolation**: User-specific data and permissions
- **Resource Sharing**: Shared resources with proper isolation

## Debugging Tests

### Test Output
```bash
# Run tests with verbose output
npx vitest --reporter=verbose

# Run specific test with debug info
npx vitest tests/utils/env.test.ts --reporter=verbose
```

### Test Debugging
```typescript
// Use console.log in tests (temporarily)
console.log('Test data:', testData);

// Use vi.fn() to track calls
const mockFn = vi.fn();
expect(mockFn).toHaveBeenCalled();

// Use expect().toMatchObject() for partial object matching
expect(result).toMatchObject({
  id: expect.any(String),
  status: 'pending',
});
```

### Common Issues
- **Mock not called**: Check if mock is properly configured
- **Async timeout**: Increase timeout or fix async issues
- **Import errors**: Verify path aliases and module resolution
- **Type errors**: Ensure proper TypeScript types in tests

## Continuous Integration

### GitHub Actions
```yaml
- name: Run tests
  run: npm run ci

- name: Upload coverage
  uses: codecov/codecov-action@v3
  with:
    file: ./coverage/lcov.info
```

### Local CI Testing
```bash
# Run full CI pipeline locally
npm run ci

# Check coverage thresholds
npm run test:coverage

# Run linting and formatting
npm run lint:fix
npm run format
```

## Contributing to Tests

When adding new features:

1. **Write tests first** (TDD approach when possible)
2. **Cover all scenarios**: happy path, error cases, edge cases
3. **Use proper mocking**: Isolate external dependencies
4. **Maintain coverage**: Keep coverage above 80%
5. **Update documentation**: Add new test categories to this README

### Test File Template
```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { YourClass } from '@/path/to/your-class';

describe('YourClass', () => {
  let instance: YourClass;

  beforeEach(() => {
    vi.clearAllMocks();
    instance = new YourClass();
  });

  describe('methodName', () => {
    it('should do what it should', async () => {
      // Arrange
      const input = 'test input';
      
      // Act
      const result = await instance.methodName(input);
      
      // Assert
      expect(result).toBe('expected output');
    });
  });
});
```

This comprehensive test suite ensures the Orbit Slack bot is reliable, maintainable, and free of regressions.
