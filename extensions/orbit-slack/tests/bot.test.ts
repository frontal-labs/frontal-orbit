import { describe, it, expect, beforeEach, vi } from 'vitest';
import { SlackBotService } from '../src/bot';

// Mock dependencies
vi.mock('../src/config', () => ({
  config: {
    slack: {
      botToken: 'xoxb-test-token',
      appToken: 'xapp-test-token',
      signingSecret: 'test-signing-secret',
    },
    app: {
      logLevel: 'error',
    },
  },
}));

vi.mock('../src/logger', () => ({
  logger: {
    error: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
  },
  logSlackEvent: vi.fn(),
}));

vi.mock('@slack/bolt', () => ({
  App: vi.fn(() => ({
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
        }),
      },
    },
  })),
}));

vi.mock('../src/api-client', () => ({
  OrbitApiClient: vi.fn(() => ({
    submitPrompt: vi.fn().mockResolvedValue({
      ok: true,
      exit_code: 0,
      args: ['prompt', 'test'],
      duration_ms: 1000,
      stdout: 'Task completed successfully',
      stderr: '',
    }),
    healthCheck: vi.fn().mockResolvedValue(true),
  })),
}));

vi.mock('../src/database', () => ({
  DatabaseService: vi.fn(() => ({
    connect: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
    healthCheck: vi.fn().mockResolvedValue(true),
    createSlackTask: vi.fn().mockResolvedValue({
      id: '123',
      slack_task_id: 'task-123',
      status: 'pending',
      request: { prompt: 'Test task' },
      created_at: new Date(),
      updated_at: new Date(),
    }),
    getSlackTask: vi.fn().mockResolvedValue(null),
    updateSlackTask: vi.fn().mockResolvedValue(null),
    getUserTasks: vi.fn().mockResolvedValue([]),
  })),
}));

vi.mock('../src/redis', () => ({
  RedisService: vi.fn(() => ({
    connect: vi.fn().mockResolvedValue(undefined),
    disconnect: vi.fn().mockResolvedValue(undefined),
    healthCheck: vi.fn().mockResolvedValue(true),
    set: vi.fn().mockResolvedValue('OK'),
    get: vi.fn().mockResolvedValue(null),
    del: vi.fn().mockResolvedValue(1),
  })),
}));

vi.mock('../src/tasks', () => ({
  TaskManager: vi.fn(() => ({
    createTask: vi.fn().mockResolvedValue({
      slack_task_id: 'task-123',
      status: 'pending',
      request: { prompt: 'Test task' },
    }),
    getTask: vi.fn().mockResolvedValue(null),
    updateTask: vi.fn().mockResolvedValue(null),
    cancelTask: vi.fn().mockResolvedValue(undefined),
    getUserTasks: vi.fn().mockResolvedValue([]),
    getTaskProgress: vi.fn().mockResolvedValue({
      task_id: 'task-123',
      status: 'pending',
      message: 'Task is pending',
      progress: 0,
      artifacts: [],
    }),
    healthCheck: vi.fn().mockResolvedValue({
      total_tasks: 0,
      active_tasks: 0,
      completed_tasks: 0,
      failed_tasks: 0,
    }),
  })),
}));

vi.mock('../src/conversations', () => ({
  ConversationManager: vi.fn(() => ({
    createContext: vi.fn().mockResolvedValue(undefined),
    getContext: vi.fn().mockResolvedValue(null),
    updateContext: vi.fn().mockResolvedValue(undefined),
    deleteContext: vi.fn().mockResolvedValue(undefined),
    healthCheck: vi.fn().mockResolvedValue(true),
  })),
}));

vi.mock('../src/validators', () => ({
  validateSlackCommand: vi.fn(() => ({ success: true, data: {} })),
  getValidationErrorMessage: vi.fn(() => ''),
}));

describe('SlackBotService', () => {
  let bot: SlackBotService;

  beforeEach(() => {
    vi.clearAllMocks();
    bot = new SlackBotService();
  });

  describe('Constructor', () => {
    it('should create bot service instance', () => {
      expect(bot).toBeInstanceOf(SlackBotService);
    });
  });

  describe('Service Methods', () => {
    it('should start successfully', async () => {
      await expect(bot.start()).resolves.not.toThrow();
    });

    it('should stop successfully', async () => {
      await expect(bot.stop()).resolves.not.toThrow();
    });

    it('should perform health check', async () => {
      const health = await bot.healthCheck();
      expect(health).toHaveProperty('slack');
      expect(health).toHaveProperty('orbit');
      expect(health).toHaveProperty('database');
      expect(health).toHaveProperty('redis');
      expect(health).toHaveProperty('tasks');
    });
  });
});
