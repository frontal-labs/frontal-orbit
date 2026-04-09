import { vi } from 'vitest';
import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';

// Mock environment variables
process.env.NODE_ENV = 'test';
process.env.LOG_LEVEL = 'error';
process.env.SLACK_BOT_TOKEN = 'xoxb-test-token';
process.env.SLACK_APP_TOKEN = 'xapp-test-token';
process.env.SLACK_SIGNING_SECRET = 'test-signing-secret-with-at-least-32-chars';
process.env.ORBIT_API_URL = 'http://localhost:8787';
process.env.DATABASE_URL = 'postgresql://test:test@localhost:5432/test_db';
process.env.REDIS_URL = 'redis://localhost:6379';

// MSW server for mocking HTTP requests
export const server = setupServer(
  // Mock Orbit API endpoints
  http.post('http://localhost:8787/v1/prompt', () => {
    return HttpResponse.json({
      ok: true,
      exit_code: 0,
      args: ['prompt', 'test'],
      duration_ms: 1000,
      stdout: 'Task completed successfully',
      stderr: '',
    });
  }),
  
  http.get('http://localhost:8787/health', () => {
    return HttpResponse.json({
      status: 'healthy',
      uptime: 3600,
      version: '1.0.0',
    });
  }),
  
  http.get('http://localhost:8787/v1/status', () => {
    return HttpResponse.json({
      system: {
        status: 'healthy',
        version: '1.0.0',
        uptime: 3600,
      },
      agents: {
        total: 1,
        active: 1,
        idle: 0,
      },
      memory: {
        used: 1024 * 1024 * 100, // 100MB
        available: 1024 * 1024 * 900, // 900MB
      },
    });
  }),
  
  http.get('http://localhost:8787/v1/sandbox', () => {
    return HttpResponse.json({
      status: 'ready',
      workspaces: 1,
      active_sessions: 0,
    });
  }),
  
  http.get('http://localhost:8787/v1/version', () => {
    return HttpResponse.json({
      version: '1.0.0',
      commit: 'abc123',
      build_time: '2024-01-01T00:00:00Z',
    });
  }),
  
  // Mock Slack API endpoints
  http.post('https://slack.com/api/auth.test', () => {
    return HttpResponse.json({
      ok: true,
      user: 'test-bot',
      team: 'test-team',
      bot_id: 'B123456',
    });
  }),
);

// Global test setup
beforeAll(() => {
  // Start MSW server
  server.listen();
  
  // Mock console methods to reduce noise in tests
  vi.spyOn(console, 'log').mockImplementation(() => {});
  vi.spyOn(console, 'info').mockImplementation(() => {});
  vi.spyOn(console, 'warn').mockImplementation(() => {});
  vi.spyOn(console, 'error').mockImplementation(() => {});
});

afterAll(() => {
  // Close MSW server
  server.close();
  
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

export const createMockSlackInteraction = (overrides = {}) => ({
  type: 'interactive_message',
  token: 'test-token',
  action_ts: '1640995200.000000',
  team: {
    id: 'T123456',
    domain: 'test-team',
  },
  user: {
    id: 'U123456',
    name: 'test-user',
  },
  channel: {
    id: 'C123456',
    name: 'general',
  },
  actions: [
    {
      name: 'task_cancel',
      type: 'button',
      value: 'task-123',
    },
  ],
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

// Mock implementations
export const mockOrbitApiClient = {
  submitPrompt: vi.fn().mockResolvedValue({
    ok: true,
    exit_code: 0,
    args: ['prompt', 'test'],
    duration_ms: 1000,
    stdout: 'Task completed successfully',
    stderr: '',
  }),
  runCliCommand: vi.fn().mockResolvedValue({
    ok: true,
    exit_code: 0,
    args: ['test'],
    duration_ms: 500,
    stdout: 'Command completed',
    stderr: '',
  }),
  healthCheck: vi.fn().mockResolvedValue(true),
  checkSandboxStatus: vi.fn().mockResolvedValue({
    status: 'ready',
    workspaces: 1,
    active_sessions: 0,
  }),
  getVersion: vi.fn().mockResolvedValue({
    version: '1.0.0',
    commit: 'abc123',
    build_time: '2024-01-01T00:00:00Z',
  }),
};

export const mockDatabaseService = {
  connect: vi.fn().mockResolvedValue(undefined),
  close: vi.fn().mockResolvedValue(undefined),
  healthCheck: vi.fn().mockResolvedValue(true),
  createSlackUser: vi.fn().mockResolvedValue(createMockSlackUser()),
  getSlackUser: vi.fn().mockResolvedValue(createMockSlackUser()),
  updateSlackUser: vi.fn().mockResolvedValue(createMockSlackUser()),
  createSlackTask: vi.fn().mockResolvedValue(createMockSlackTask()),
  getSlackTask: vi.fn().mockResolvedValue(createMockSlackTask()),
  updateSlackTask: vi.fn().mockResolvedValue(createMockSlackTask()),
  getUserTasks: vi.fn().mockResolvedValue([createMockSlackTask()]),
  deleteSlackTask: vi.fn().mockResolvedValue(undefined),
  createConversationContext: vi.fn().mockResolvedValue(undefined),
  getConversationContext: vi.fn().mockResolvedValue(undefined),
  updateConversationContext: vi.fn().mockResolvedValue(undefined),
  deleteConversationContext: vi.fn().mockResolvedValue(undefined),
};

export const mockRedisService = {
  connect: vi.fn().mockResolvedValue(undefined),
  disconnect: vi.fn().mockResolvedValue(undefined),
  healthCheck: vi.fn().mockResolvedValue(true),
  set: vi.fn().mockResolvedValue('OK'),
  get: vi.fn().mockResolvedValue('test-value'),
  del: vi.fn().mockResolvedValue(1),
  exists: vi.fn().mockResolvedValue(true),
  expire: vi.fn().mockResolvedValue(true),
  keys: vi.fn().mockResolvedValue(['key1', 'key2']),
  flushAll: vi.fn().mockResolvedValue('OK'),
  cacheTask: vi.fn().mockResolvedValue(undefined),
  getCachedTask: vi.fn().mockResolvedValue(createMockSlackTask()),
  deleteCachedTask: vi.fn().mockResolvedValue(undefined),
  setUserPreferences: vi.fn().mockResolvedValue(undefined),
  getUserPreferences: vi.fn().mockResolvedValue({}),
  deleteUserPreferences: vi.fn().mockResolvedValue(undefined),
  setRateLimit: vi.fn().mockResolvedValue(undefined),
  checkRateLimit: vi.fn().mockResolvedValue(true),
  addToQueue: vi.fn().mockResolvedValue(undefined),
  getFromQueue: vi.fn().mockResolvedValue({}),
  removeFromQueue: vi.fn().mockResolvedValue(undefined),
};

export const mockTaskManager = {
  createTask: vi.fn().mockResolvedValue(createMockSlackTask()),
  getTask: vi.fn().mockResolvedValue(createMockSlackTask()),
  updateTask: vi.fn().mockResolvedValue(createMockSlackTask()),
  cancelTask: vi.fn().mockResolvedValue(undefined),
  getUserTasks: vi.fn().mockResolvedValue([createMockSlackTask()]),
  getTaskProgress: vi.fn().mockResolvedValue({
    task_id: 'task-123',
    status: 'running',
    message: 'Task is in progress',
    progress: 50,
    artifacts: [],
  }),
  healthCheck: vi.fn().mockResolvedValue({
    total_tasks: 10,
    active_tasks: 3,
    completed_tasks: 7,
    failed_tasks: 0,
  }),
};

export const mockConversationManager = {
  createContext: vi.fn().mockResolvedValue(undefined),
  getContext: vi.fn().mockResolvedValue({
    channel_id: 'C123456',
    thread_ts: '1640995200.000000',
    user_id: 'U123456',
    context: {
      current_task: 'task-123',
      repository: 'test-org/test-repo',
      branch: 'main',
      last_command: '/orbit-create',
      preferences: {},
    },
    created_at: new Date(),
    updated_at: new Date(),
  }),
  updateContext: vi.fn().mockResolvedValue(undefined),
  deleteContext: vi.fn().mockResolvedValue(undefined),
  healthCheck: vi.fn().mockResolvedValue(true),
};

// Mock Slack Bolt app
export const mockSlackApp = {
  command: vi.fn(),
  message: vi.fn(),
  action: vi.fn(),
  event: vi.fn(),
  start: vi.fn().mockResolvedValue(undefined),
  stop: vi.fn().mockResolvedValue(undefined),
  client: {
    auth: {
      test: vi.fn().mockResolvedValue({
        ok: true,
        user: 'test-bot',
        team: 'test-team',
        bot_id: 'B123456',
      }),
    },
    chat: {
      postMessage: vi.fn().mockResolvedValue({
        ok: true,
        channel: 'C123456',
        ts: '1640995200.000000',
      }),
    },
  },
};
