import { describe, it, expect, beforeEach, vi } from 'vitest';
import { SlackBotService } from '@/bot/slack-bot-service';
import {
  createMockSlackCommand,
  createMockSlackInteraction,
  createMockTaskCreationRequest,
  mockOrbitApiClient,
  mockDatabaseService,
  mockRedisService,
  mockTaskManager,
  mockConversationManager,
  mockSlackApp,
} from '../setup';

// Mock the dependencies
vi.mock('@/services/orbit-api-client', () => ({
  OrbitApiClient: vi.fn(() => mockOrbitApiClient),
}));

vi.mock('@/services/database-service', () => ({
  DatabaseService: vi.fn(() => mockDatabaseService),
}));

vi.mock('@/services/redis-service', () => ({
  RedisService: vi.fn(() => mockRedisService),
}));

vi.mock('@/services/task-manager', () => ({
  TaskManager: vi.fn(() => mockTaskManager),
}));

vi.mock('@/services/conversation-manager', () => ({
  ConversationManager: vi.fn(() => mockConversationManager),
}));

vi.mock('@/utils/config', () => ({
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

vi.mock('@/utils/logger', () => ({
  logger: {
    error: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
  },
  logSlackEvent: vi.fn(),
}));

vi.mock('@slack/bolt', () => ({
  App: vi.fn(() => mockSlackApp),
}));

describe('SlackBotService', () => {
  let slackBotService: SlackBotService;

  beforeEach(() => {
    vi.clearAllMocks();
    slackBotService = new SlackBotService();
  });

  describe('Constructor', () => {
    it('should initialize the Slack bot service', () => {
      expect(slackBotService).toBeInstanceOf(SlackBotService);
    });

    it('should register all handlers', () => {
      expect(mockSlackApp.command).toHaveBeenCalledWith('/orbit-create', expect.any(Function));
      expect(mockSlackApp.command).toHaveBeenCalledWith('/orbit-status', expect.any(Function));
      expect(mockSlackApp.command).toHaveBeenCalledWith('/orbit-list', expect.any(Function));
      expect(mockSlackApp.command).toHaveBeenCalledWith('/orbit-pause', expect.any(Function));
      expect(mockSlackApp.command).toHaveBeenCalledWith('/orbit-resume', expect.any(Function));
      expect(mockSlackApp.command).toHaveBeenCalledWith('/orbit-cancel', expect.any(Function));
      expect(mockSlackApp.command).toHaveBeenCalledWith('/orbit-help', expect.any(Function));

      expect(mockSlackApp.message).toHaveBeenCalledWith('hello orbit', expect.any(Function));
      expect(mockSlackApp.message).toHaveBeenCalledWith(/orbit (.+)/, expect.any(Function));

      expect(mockSlackApp.action).toHaveBeenCalledWith('task_pause', expect.any(Function));
      expect(mockSlackApp.action).toHaveBeenCalledWith('task_resume', expect.any(Function));
      expect(mockSlackApp.action).toHaveBeenCalledWith('task_cancel', expect.any(Function));

      expect(mockSlackApp.event).toHaveBeenCalledWith('app_mention', expect.any(Function));
    });
  });

  describe('Command Handling', () => {
    describe('Create Command', () => {
      it('should handle valid create command successfully', async () => {
        const command = createMockSlackCommand({
          text: 'Fix the login bug',
        });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        // Mock successful task creation
        const mockTask = {
          slack_task_id: 'task-123',
          status: 'pending',
          request: createMockTaskCreationRequest({ prompt: 'Fix the login bug' }),
          created_at: new Date(),
          updated_at: new Date(),
        };
        mockTaskManager.createTask.mockResolvedValue(mockTask);

        // Get the handler function and call it
        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-create'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Task created successfully!',
          blocks: expect.arrayContaining([
            expect.objectContaining({
              type: 'section',
              text: expect.objectContaining({
                text: expect.stringContaining('task-123'),
              }),
            }),
          ]),
          response_type: 'in_channel',
        });
      });

      it('should handle invalid command with validation error', async () => {
        const invalidCommand = createMockSlackCommand({ token: '' });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-create'
        )?.[1];
        await handler({ command: invalidCommand, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: expect.stringContaining('Invalid command:'),
          response_type: 'ephemeral',
        });
      });

      it('should handle empty command text with usage help', async () => {
        const command = createMockSlackCommand({ text: '' });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-create'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: expect.stringContaining('Usage:'),
          response_type: 'ephemeral',
        });
      });

      it('should handle task creation errors gracefully', async () => {
        const command = createMockSlackCommand({ text: 'Test task' });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        // Mock task creation error
        const error = new Error('Task creation failed');
        mockTaskManager.createTask.mockRejectedValue(error);

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-create'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Failed to create task: Task creation failed',
          response_type: 'ephemeral',
        });
      });

      it('should parse task creation request with flags', async () => {
        const command = createMockSlackCommand({
          text: 'Add user profile page --repository myorg/myapp --branch feature/profile --model claude-3-sonnet --provider anthropic',
        });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const mockTask = {
          slack_task_id: 'task-123',
          status: 'pending',
          request: expect.objectContaining({
            prompt: 'Add user profile page',
            repository: 'myorg/myapp',
            branch: 'feature/profile',
            model: 'claude-3-sonnet',
            provider: 'anthropic',
          }),
          created_at: new Date(),
          updated_at: new Date(),
        };
        mockTaskManager.createTask.mockResolvedValue(mockTask);

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-create'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockTaskManager.createTask).toHaveBeenCalledWith(
          command.user_id,
          expect.objectContaining({
            prompt: 'Add user profile page',
            repository: 'myorg/myapp',
            branch: 'feature/profile',
            model: 'claude-3-sonnet',
            provider: 'anthropic',
          })
        );
      });
    });

    describe('Status Command', () => {
      it('should handle status command with specific task ID', async () => {
        const command = createMockSlackCommand({ text: 'task-123' });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const mockTask = {
          slack_task_id: 'task-123',
          status: 'running',
          request: createMockTaskCreationRequest(),
          created_at: new Date(),
          updated_at: new Date(),
        };
        mockTaskManager.getTask.mockResolvedValue(mockTask);
        mockTaskManager.getTaskProgress.mockResolvedValue({
          task_id: 'task-123',
          status: 'running',
          message: 'Task is in progress',
          progress: 50,
          artifacts: [],
        });

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-status'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          blocks: expect.arrayContaining([
            expect.objectContaining({
              text: expect.objectContaining({
                text: expect.stringContaining('task-123'),
              }),
            }),
          ]),
          response_type: 'ephemeral',
        });
      });

      it('should handle status command without task ID (show most recent)', async () => {
        const command = createMockSlackCommand({ text: '' });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const mockTask = {
          slack_task_id: 'task-456',
          status: 'completed',
          request: createMockTaskCreationRequest(),
          created_at: new Date(),
          updated_at: new Date(),
        };
        mockTaskManager.getUserTasks.mockResolvedValue([mockTask]);
        mockTaskManager.getTaskProgress.mockResolvedValue({
          task_id: 'task-456',
          status: 'completed',
          message: 'Task completed successfully',
          progress: 100,
          artifacts: [],
        });

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-status'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          blocks: expect.arrayContaining([
            expect.objectContaining({
              text: expect.objectContaining({
                text: expect.stringContaining('task-456'),
              }),
            }),
          ]),
          response_type: 'ephemeral',
        });
      });

      it('should handle task not found', async () => {
        const command = createMockSlackCommand({ text: 'nonexistent-task' });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        mockTaskManager.getTask.mockResolvedValue(null);

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-status'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Task `nonexistent-task` not found',
          response_type: 'ephemeral',
        });
      });

      it('should handle no tasks found for user', async () => {
        const command = createMockSlackCommand({ text: '' });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        mockTaskManager.getUserTasks.mockResolvedValue([]);

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-status'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'No tasks found. Use `/orbit-create` to create a new task.',
          response_type: 'ephemeral',
        });
      });
    });

    describe('List Command', () => {
      it('should handle list command with tasks', async () => {
        const command = createMockSlackCommand();
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const mockTasks = [
          {
            slack_task_id: 'task-1',
            status: 'completed',
            request: createMockTaskCreationRequest({ prompt: 'First task' }),
            created_at: new Date(),
            updated_at: new Date(),
          },
          {
            slack_task_id: 'task-2',
            status: 'running',
            request: createMockTaskCreationRequest({ prompt: 'Second task' }),
            created_at: new Date(),
            updated_at: new Date(),
          },
        ];
        mockTaskManager.getUserTasks.mockResolvedValue(mockTasks);

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-list'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          blocks: expect.arrayContaining([
            expect.objectContaining({
              text: expect.objectContaining({
                text: 'Your Recent Tasks',
              }),
            }),
          ]),
          response_type: 'ephemeral',
        });
      });

      it('should handle list command with no tasks', async () => {
        const command = createMockSlackCommand();
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        mockTaskManager.getUserTasks.mockResolvedValue([]);

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-list'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'No tasks found. Use `/orbit-create` to create a new task.',
          response_type: 'ephemeral',
        });
      });
    });

    describe('Cancel Command', () => {
      it('should handle cancel command successfully', async () => {
        const command = createMockSlackCommand({ text: 'task-123' });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        mockTaskManager.cancelTask.mockResolvedValue(undefined);

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-cancel'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockTaskManager.cancelTask).toHaveBeenCalledWith('task-123', command.user_id);
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Task `task-123` has been cancelled.',
          response_type: 'in_channel',
        });
      });

      it('should handle cancel command without task ID', async () => {
        const command = createMockSlackCommand({ text: '' });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-cancel'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Usage: `/orbit-cancel <task_id>`',
          response_type: 'ephemeral',
        });
      });

      it('should handle cancel command errors', async () => {
        const command = createMockSlackCommand({ text: 'task-123' });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const error = new Error('Task not found');
        mockTaskManager.cancelTask.mockRejectedValue(error);

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-cancel'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Failed to cancel task: Task not found',
          response_type: 'ephemeral',
        });
      });
    });

    describe('Help Command', () => {
      it('should handle help command', async () => {
        const command = createMockSlackCommand();
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-help'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          blocks: expect.arrayContaining([
            expect.objectContaining({
              text: expect.objectContaining({
                text: 'Orbit Slack Bot Help',
              }),
            }),
            expect.objectContaining({
              text: expect.objectContaining({
                text: expect.stringContaining('Available Commands:'),
              }),
            }),
          ]),
          response_type: 'ephemeral',
        });
      });
    });

    describe('Pause/Resume Commands', () => {
      it('should handle pause command with not implemented message', async () => {
        const command = createMockSlackCommand();
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-pause'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Pause functionality is not yet implemented.',
          response_type: 'ephemeral',
        });
      });

      it('should handle resume command with not implemented message', async () => {
        const command = createMockSlackCommand();
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const handler = mockSlackApp.command.mock.calls.find(
          ([cmd]) => cmd === '/orbit-resume'
        )?.[1];
        await handler({ command, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Resume functionality is not yet implemented.',
          response_type: 'ephemeral',
        });
      });
    });
  });

  describe('Message Handling', () => {
    describe('Hello Message', () => {
      it('should handle hello orbit message', async () => {
        const message = {
          user: 'U123456',
          channel: 'C123456',
          text: 'hello orbit',
        };
        const mockSay = vi.fn();

        const handler = mockSlackApp.message.mock.calls.find(
          ([pattern]) => pattern === 'hello orbit'
        )?.[1];
        await handler({ message, say: mockSay });

        expect(mockSay).toHaveBeenCalledWith(
          'Hello! I\'m Orbit, your autonomous coding assistant. Use `/orbit-help` to see what I can do!'
        );
      });
    });

    describe('Direct Message', () => {
      it('should handle direct message with task creation', async () => {
        const message = {
          user: 'U123456',
          channel: 'C123456',
          text: 'orbit fix the login bug',
        };
        const mockSay = vi.fn();

        const mockTask = {
          slack_task_id: 'task-789',
          status: 'pending',
          request: createMockTaskCreationRequest({ prompt: 'fix the login bug' }),
          created_at: new Date(),
          updated_at: new Date(),
        };
        mockTaskManager.createTask.mockResolvedValue(mockTask);

        const handler = mockSlackApp.message.mock.calls.find(
          ([pattern]) => pattern instanceof RegExp && pattern.test('orbit fix the login bug')
        )?.[1];
        await handler({ message, say: mockSay });

        expect(mockSay).toHaveBeenCalledWith(
          'Task created: `task-789`. I\'ll start working on it right away!'
        );
      });

      it('should handle direct message without task text', async () => {
        const message = {
          user: 'U123456',
          channel: 'C123456',
          text: 'orbit',
        };
        const mockSay = vi.fn();

        const handler = mockSlackApp.message.mock.calls.find(
          ([pattern]) => pattern instanceof RegExp && pattern.test('orbit')
        )?.[1];
        await handler({ message, say: mockSay });

        expect(mockSay).toHaveBeenCalledWith('Please provide a task description after "orbit".');
      });

      it('should handle direct message errors', async () => {
        const message = {
          user: 'U123456',
          channel: 'C123456',
          text: 'orbit test task',
        };
        const mockSay = vi.fn();

        const error = new Error('Failed to create task');
        mockTaskManager.createTask.mockRejectedValue(error);

        const handler = mockSlackApp.message.mock.calls.find(
          ([pattern]) => pattern instanceof RegExp && pattern.test('orbit test task')
        )?.[1];
        await handler({ message, say: mockSay });

        expect(mockSay).toHaveBeenCalledWith('Sorry, I couldn\'t create that task: Failed to create task');
      });
    });
  });

  describe('Interaction Handling', () => {
    describe('Task Cancel Action', () => {
      it('should handle task cancel action', async () => {
        const interaction = createMockSlackInteraction({
          actions: [
            {
              name: 'task_cancel',
              type: 'button',
              value: 'task-123',
            },
          ],
        });
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        mockTaskManager.cancelTask.mockResolvedValue(undefined);

        const handler = mockSlackApp.action.mock.calls.find(
          ([action]) => action === 'task_cancel'
        )?.[1];
        await handler({ body: interaction, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockTaskManager.cancelTask).toHaveBeenCalledWith('task-123', interaction.user.id);
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Task `task-123` has been cancelled.',
          replace_original: true,
        });
      });

      it('should handle task cancel action errors', async () => {
        const interaction = createMockSlackInteraction();
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const error = new Error('Task not found');
        mockTaskManager.cancelTask.mockRejectedValue(error);

        const handler = mockSlackApp.action.mock.calls.find(
          ([action]) => action === 'task_cancel'
        )?.[1];
        await handler({ body: interaction, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Failed to cancel task: Task not found',
          response_type: 'ephemeral',
        });
      });
    });

    describe('Task Pause/Resume Actions', () => {
      it('should handle task pause action with not implemented message', async () => {
        const interaction = createMockSlackInteraction();
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const handler = mockSlackApp.action.mock.calls.find(
          ([action]) => action === 'task_pause'
        )?.[1];
        await handler({ body: interaction, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Pause functionality is not yet implemented.',
          response_type: 'ephemeral',
        });
      });

      it('should handle task resume action with not implemented message', async () => {
        const interaction = createMockSlackInteraction();
        const mockAck = vi.fn();
        const mockRespond = vi.fn();

        const handler = mockSlackApp.action.mock.calls.find(
          ([action]) => action === 'task_resume'
        )?.[1];
        await handler({ body: interaction, ack: mockAck, respond: mockRespond });

        expect(mockAck).toHaveBeenCalled();
        expect(mockRespond).toHaveBeenCalledWith({
          text: 'Resume functionality is not yet implemented.',
          response_type: 'ephemeral',
        });
      });
    });
  });

  describe('Event Handling', () => {
    describe('App Mention', () => {
      it('should handle app mention with task creation', async () => {
        const event = {
          user: 'U123456',
          channel: 'C123456',
          text: '<@U789012> can you help me fix this bug?',
        };
        const mockSay = vi.fn();

        const mockTask = {
          slack_task_id: 'task-999',
          status: 'pending',
          request: createMockTaskCreationRequest({ prompt: 'can you help me fix this bug?' }),
          created_at: new Date(),
          updated_at: new Date(),
        };
        mockTaskManager.createTask.mockResolvedValue(mockTask);

        const handler = mockSlackApp.event.mock.calls.find(
          ([event_type]) => event_type === 'app_mention'
        )?.[1];
        await handler({ event, say: mockSay });

        expect(mockSay).toHaveBeenCalledWith(
          'Task created: `task-999`. I\'ll start working on it right away!'
        );
      });

      it('should handle app mention without task text', async () => {
        const event = {
          user: 'U123456',
          channel: 'C123456',
          text: '<@U789012>',
        };
        const mockSay = vi.fn();

        const handler = mockSlackApp.event.mock.calls.find(
          ([event_type]) => event_type === 'app_mention'
        )?.[1];
        await handler({ event, say: mockSay });

        expect(mockSay).toHaveBeenCalledWith('Hi! How can I help you with your coding tasks?');
      });

      it('should handle app mention errors', async () => {
        const event = {
          user: 'U123456',
          channel: 'C123456',
          text: '<@U789012> test task',
        };
        const mockSay = vi.fn();

        const error = new Error('Failed to create task');
        mockTaskManager.createTask.mockRejectedValue(error);

        const handler = mockSlackApp.event.mock.calls.find(
          ([event_type]) => event_type === 'app_mention'
        )?.[1];
        await handler({ event, say: mockSay });

        expect(mockSay).toHaveBeenCalledWith('Sorry, I couldn\'t create that task: Failed to create task');
      });
    });
  });

  describe('Service Methods', () => {
    describe('start', () => {
      it('should start the bot service successfully', async () => {
        mockRedisService.connect.mockResolvedValue(undefined);
        mockSlackApp.start.mockResolvedValue(undefined);

        await slackBotService.start();

        expect(mockRedisService.connect).toHaveBeenCalled();
        expect(mockSlackApp.start).toHaveBeenCalled();
      });

      it('should handle start errors', async () => {
        const error = new Error('Failed to start bot');
        mockRedisService.connect.mockRejectedValue(error);

        await expect(slackBotService.start()).rejects.toThrow('Failed to start bot');
      });
    });

    describe('stop', () => {
      it('should stop the bot service successfully', async () => {
        mockSlackApp.stop.mockResolvedValue(undefined);
        mockRedisService.disconnect.mockResolvedValue(undefined);
        mockDatabaseService.close.mockResolvedValue(undefined);

        await slackBotService.stop();

        expect(mockSlackApp.stop).toHaveBeenCalled();
        expect(mockRedisService.disconnect).toHaveBeenCalled();
        expect(mockDatabaseService.close).toHaveBeenCalled();
      });

      it('should handle stop errors', async () => {
        const error = new Error('Failed to stop bot');
        mockSlackApp.stop.mockRejectedValue(error);

        await expect(slackBotService.stop()).rejects.toThrow('Failed to stop bot');
      });
    });

    describe('healthCheck', () => {
      it('should return health status for all services', async () => {
        mockSlackApp.client.auth.test.mockResolvedValue({
          ok: true,
          user: 'test-bot',
        });
        mockOrbitApiClient.healthCheck.mockResolvedValue(true);
        mockDatabaseService.healthCheck.mockResolvedValue(true);
        mockRedisService.healthCheck.mockResolvedValue(true);
        mockTaskManager.healthCheck.mockResolvedValue({
          total_tasks: 10,
          active_tasks: 3,
          completed_tasks: 7,
          failed_tasks: 0,
        });

        const health = await slackBotService.healthCheck();

        expect(health).toEqual({
          slack: true,
          orbit: true,
          database: true,
          redis: true,
          tasks: {
            total_tasks: 10,
            active_tasks: 3,
            completed_tasks: 7,
            failed_tasks: 0,
          },
        });
      });

      it('should handle service failures in health check', async () => {
        mockSlackApp.client.auth.test.mockRejectedValue(new Error('Slack API error'));
        mockOrbitApiClient.healthCheck.mockResolvedValue(false);
        mockDatabaseService.healthCheck.mockResolvedValue(false);
        mockRedisService.healthCheck.mockResolvedValue(false);
        mockTaskManager.healthCheck.mockResolvedValue({
          total_tasks: 0,
          active_tasks: 0,
          completed_tasks: 0,
          failed_tasks: 0,
        });

        const health = await slackBotService.healthCheck();

        expect(health).toEqual({
          slack: false,
          orbit: false,
          database: false,
          redis: false,
          tasks: {
            total_tasks: 0,
            active_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
          },
        });
      });
    });
  });
});
