import { vi } from 'vitest';

// Mock environment variables
process.env.NODE_ENV = 'test';
process.env.LOG_LEVEL = 'error';
process.env.SLACK_BOT_TOKEN = 'xoxb-test-token';
process.env.SLACK_APP_TOKEN = 'xapp-test-token';
process.env.SLACK_SIGNING_SECRET = 'test-signing-secret-with-at-least-32-chars';
process.env.ORBIT_API_URL = 'http://localhost:8787';
process.env.DATABASE_URL = 'postgresql://test:test@localhost:5432/test_db';
process.env.REDIS_URL = 'redis://localhost:6379';

// Global test setup
beforeAll(() => {
  // Mock console methods to reduce noise in tests
  vi.spyOn(console, 'log').mockImplementation(() => { });
  vi.spyOn(console, 'info').mockImplementation(() => { });
  vi.spyOn(console, 'warn').mockImplementation(() => { });
  vi.spyOn(console, 'error').mockImplementation(() => { });
});

afterAll(() => {
  // Restore console mocks
  vi.restoreAllMocks();
});

// Reset mocks after each test
afterEach(() => {
  vi.clearAllMocks();
});

// Global test utilities
export const createMockSlackCommand = (overrides = {}) => ({
  token: 'test-token',
  team_id: 'T123456',
  team_domain: 'test-team',
  channel_id: 'C123456',
  channel_name: 'general',
  user_id: 'U123456',
  user_name: 'test-user',
  command: '/orbit-create',
  text: 'Test task description',
  response_url: 'https://hooks.slack.com/test',
  trigger_id: 'trigger-123',
  ...overrides,
});

export const createMockTaskCreationRequest = (overrides = {}) => ({
  prompt: 'Test task description',
  repository: 'test-org/test-repo',
  branch: 'main',
  model: 'claude-3-sonnet',
  provider: 'anthropic',
  permission_mode: 'auto',
  allowed_tools: ['file_system', 'git'],
  priority: 'medium',
  ...overrides,
});

export const createMockSlackTask = (overrides = {}) => ({
  id: '123',
  slack_task_id: 'task-123',
  orbit_task_id: 'orbit-123',
  user_id: 'U123456',
  status: 'pending',
  request: createMockTaskCreationRequest(),
  created_at: new Date(),
  updated_at: new Date(),
  ...overrides,
});

export const createMockSlackUser = (overrides = {}) => ({
  id: '123',
  slack_user_id: 'U123456',
  name: 'Test User',
  real_name: 'Test User',
  preferences: {
    default_model: 'claude-3-sonnet',
    default_provider: 'anthropic',
    notification_level: 'important',
    auto_merge: false,
  },
  permissions: {
    can_create_tasks: true,
    can_cancel_tasks: true,
    can_view_all_tasks: false,
    repositories: ['test-org/test-repo'],
  },
  created_at: new Date(),
  updated_at: new Date(),
  ...overrides,
});
