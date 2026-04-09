import { describe, it, expect, beforeEach, vi } from 'vitest';
import { SlackBotService } from '@/bot/slack-bot-service';
import { server } from '../setup';

describe('Slack Workflow Integration Tests', () => {
  let slackBotService: SlackBotService;

  beforeEach(() => {
    vi.clearAllMocks();
    slackBotService = new SlackBotService();
  });

  describe('Complete Task Creation Workflow', () => {
    it('should handle the full task creation lifecycle', async () => {
      // Step 1: User creates a task via slash command
      const command = {
        token: 'xoxb-test-token',
        team_id: 'T123456',
        team_domain: 'test-team',
        channel_id: 'C123456',
        channel_name: 'general',
        user_id: 'U123456',
        user_name: 'test-user',
        command: '/orbit-create',
        text: 'Fix the authentication bug in the login service',
        response_url: 'https://hooks.slack.com/test',
        trigger_id: 'trigger-123',
      };

      const mockAck = vi.fn();
      const mockRespond = vi.fn();

      // Mock successful task creation
      const mockTask = {
        slack_task_id: 'task-123',
        status: 'pending',
        request: {
          prompt: 'Fix the authentication bug in the login service',
          repository: undefined,
          branch: undefined,
          model: undefined,
          provider: undefined,
          permission_mode: undefined,
          allowed_tools: undefined,
          priority: 'medium',
        },
        created_at: new Date(),
        updated_at: new Date(),
      };

      // Mock the task manager to return the created task
      const { mockTaskManager } = await import('../setup');
      mockTaskManager.createTask.mockResolvedValue(mockTask);

      // Get the create command handler
      const { mockSlackApp } = await import('../setup');
      const createHandler = mockSlackApp.command.mock.calls.find(
        ([cmd]) => cmd === '/orbit-create'
      )?.[1];

      // Execute the command
      await createHandler({ command, ack: mockAck, respond: mockRespond });

      // Verify the command was acknowledged
      expect(mockAck).toHaveBeenCalled();

      // Verify the response contains task creation confirmation
      expect(mockRespond).toHaveBeenCalledWith({
        text: 'Task created successfully!',
        blocks: expect.arrayContaining([
          expect.objectContaining({
            type: 'section',
            text: expect.objectContaining({
              text: expect.stringContaining('task-123'),
            }),
          }),
          expect.objectContaining({
            type: 'actions',
            elements: expect.arrayContaining([
              expect.objectContaining({
                action_id: 'task_cancel',
                value: 'task-123',
              }),
            ]),
          }),
        ]),
        response_type: 'in_channel',
      });

      // Step 2: User checks task status
      const statusCommand = {
        ...command,
        command: '/orbit-status',
        text: 'task-123',
      };

      const mockStatusAck = vi.fn();
      const mockStatusRespond = vi.fn();

      // Mock task status response
      mockTaskManager.getTask.mockResolvedValue(mockTask);
      mockTaskManager.getTaskProgress.mockResolvedValue({
        task_id: 'task-123',
        status: 'running',
        message: 'Task is in progress - analyzing the authentication code',
        progress: 25,
        artifacts: [],
      });

      // Get the status command handler
      const statusHandler = mockSlackApp.command.mock.calls.find(
        ([cmd]) => cmd === '/orbit-status'
      )?.[1];

      // Execute the status command
      await statusHandler({ command: statusCommand, ack: mockStatusAck, respond: mockStatusRespond });

      // Verify status response
      expect(mockStatusRespond).toHaveBeenCalledWith({
        blocks: expect.arrayContaining([
          expect.objectContaining({
            text: expect.objectContaining({
              text: expect.stringContaining('task-123'),
            }),
          }),
          expect.objectContaining({
            text: expect.objectContaining({
              text: expect.stringContaining('running'),
            }),
          }),
          expect.objectContaining({
            text: expect.objectContaining({
              text: expect.stringContaining('25%'),
            }),
          }),
        ]),
        response_type: 'ephemeral',
      });

      // Step 3: User cancels the task
      const cancelCommand = {
        ...command,
        command: '/orbit-cancel',
        text: 'task-123',
      };

      const mockCancelAck = vi.fn();
      const mockCancelRespond = vi.fn();

      // Mock successful cancellation
      mockTaskManager.cancelTask.mockResolvedValue(undefined);

      // Get the cancel command handler
      const cancelHandler = mockSlackApp.command.mock.calls.find(
        ([cmd]) => cmd === '/orbit-cancel'
      )?.[1];

      // Execute the cancel command
      await cancelHandler({ command: cancelCommand, ack: mockCancelAck, respond: mockCancelRespond });

      // Verify cancellation response
      expect(mockCancelRespond).toHaveBeenCalledWith({
        text: 'Task `task-123` has been cancelled.',
        response_type: 'in_channel',
      });
    });
  });

  describe('Error Handling Workflow', () => {
    it('should handle validation errors gracefully', async () => {
      const invalidCommand = {
        token: '', // Invalid token
        team_id: 'T123456',
        team_domain: 'test-team',
        channel_id: 'C123456',
        channel_name: 'general',
        user_id: 'U123456',
        user_name: 'test-user',
        command: '/orbit-create',
        text: 'Test task',
        response_url: 'https://hooks.slack.com/test',
        trigger_id: 'trigger-123',
      };

      const mockAck = vi.fn();
      const mockRespond = vi.fn();

      // Get the create command handler
      const { mockSlackApp } = await import('../setup');
      const handler = mockSlackApp.command.mock.calls.find(
        ([cmd]) => cmd === '/orbit-create'
      )?.[1];

      // Execute the invalid command
      await handler({ command: invalidCommand, ack: mockAck, respond: mockRespond });

      // Verify validation error response
      expect(mockAck).toHaveBeenCalled();
      expect(mockRespond).toHaveBeenCalledWith({
        text: expect.stringContaining('Invalid command:'),
        response_type: 'ephemeral',
      });
    });

    it('should handle service errors gracefully', async () => {
      const command = {
        token: 'xoxb-test-token',
        team_id: 'T123456',
        team_domain: 'test-team',
        channel_id: 'C123456',
        channel_name: 'general',
        user_id: 'U123456',
        user_name: 'test-user',
        command: '/orbit-create',
        text: 'Test task',
        response_url: 'https://hooks.slack.com/test',
        trigger_id: 'trigger-123',
      };

      const mockAck = vi.fn();
      const mockRespond = vi.fn();

      // Mock service error
      const { mockTaskManager } = await import('../setup');
      mockTaskManager.createTask.mockRejectedValue(new Error('Service unavailable'));

      // Get the create command handler
      const { mockSlackApp } = await import('../setup');
      const handler = mockSlackApp.command.mock.calls.find(
        ([cmd]) => cmd === '/orbit-create'
      )?.[1];

      // Execute the command
      await handler({ command, ack: mockAck, respond: mockRespond });

      // Verify error response
      expect(mockAck).toHaveBeenCalled();
      expect(mockRespond).toHaveBeenCalledWith({
        text: 'Failed to create task: Service unavailable',
        response_type: 'ephemeral',
      });
    });
  });

  describe('Multi-User Interaction Workflow', () => {
    it('should handle multiple users creating tasks simultaneously', async () => {
      const user1Command = {
        token: 'xoxb-test-token',
        team_id: 'T123456',
        team_domain: 'test-team',
        channel_id: 'C123456',
        channel_name: 'general',
        user_id: 'U123456',
        user_name: 'user1',
        command: '/orbit-create',
        text: 'User 1 task',
        response_url: 'https://hooks.slack.com/test1',
        trigger_id: 'trigger-123',
      };

      const user2Command = {
        ...user1Command,
        user_id: 'U789012',
        user_name: 'user2',
        text: 'User 2 task',
        response_url: 'https://hooks.slack.com/test2',
        trigger_id: 'trigger-456',
      };

      const mockAck1 = vi.fn();
      const mockRespond1 = vi.fn();
      const mockAck2 = vi.fn();
      const mockRespond2 = vi.fn();

      // Mock different tasks for each user
      const { mockTaskManager } = await import('../setup');
      mockTaskManager.createTask
        .mockResolvedValueOnce({
          slack_task_id: 'task-user1-123',
          status: 'pending',
          request: { prompt: 'User 1 task', priority: 'medium' },
          created_at: new Date(),
          updated_at: new Date(),
        })
        .mockResolvedValueOnce({
          slack_task_id: 'task-user2-456',
          status: 'pending',
          request: { prompt: 'User 2 task', priority: 'medium' },
          created_at: new Date(),
          updated_at: new Date(),
        });

      // Get the create command handler
      const { mockSlackApp } = await import('../setup');
      const handler = mockSlackApp.command.mock.calls.find(
        ([cmd]) => cmd === '/orbit-create'
      )?.[1];

      // Execute both commands concurrently
      await Promise.all([
        handler({ command: user1Command, ack: mockAck1, respond: mockRespond1 }),
        handler({ command: user2Command, ack: mockAck2, respond: mockRespond2 }),
      ]);

      // Verify both commands were handled
      expect(mockAck1).toHaveBeenCalled();
      expect(mockAck2).toHaveBeenCalled();
      expect(mockRespond1).toHaveBeenCalled();
      expect(mockRespond2).toHaveBeenCalled();

      // Verify different tasks were created
      expect(mockTaskManager.createTask).toHaveBeenCalledWith('U123456', expect.any(Object));
      expect(mockTaskManager.createTask).toHaveBeenCalledWith('U789012', expect.any(Object));
    });
  });

  describe('Message and Interaction Workflow', () => {
    it('should handle natural language task creation via mention', async () => {
      const mentionEvent = {
        user: 'U123456',
        channel: 'C123456',
        text: '<@U789012> can you help me fix the login bug?',
      };

      const mockSay = vi.fn();

      // Mock task creation
      const { mockTaskManager } = await import('../setup');
      mockTaskManager.createTask.mockResolvedValue({
        slack_task_id: 'task-mention-123',
        status: 'pending',
        request: { prompt: 'can you help me fix the login bug?', priority: 'medium' },
        created_at: new Date(),
        updated_at: new Date(),
      });

      // Get the app mention handler
      const { mockSlackApp } = await import('../setup');
      const handler = mockSlackApp.event.mock.calls.find(
        ([event_type]) => event_type === 'app_mention'
      )?.[1];

      // Execute the mention handler
      await handler({ event: mentionEvent, say: mockSay });

      // Verify task creation response
      expect(mockSay).toHaveBeenCalledWith(
        'Task created: `task-mention-123`. I\'ll start working on it right away!'
      );
    });

    it('should handle task cancellation via button interaction', async () => {
      const interaction = {
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
            value: 'task-cancel-123',
          },
        ],
      };

      const mockAck = vi.fn();
      const mockRespond = vi.fn();

      // Mock successful cancellation
      const { mockTaskManager } = await import('../setup');
      mockTaskManager.cancelTask.mockResolvedValue(undefined);

      // Get the task cancel action handler
      const { mockSlackApp } = await import('../setup');
      const handler = mockSlackApp.action.mock.calls.find(
        ([action]) => action === 'task_cancel'
      )?.[1];

      // Execute the interaction handler
      await handler({ body: interaction, ack: mockAck, respond: mockRespond });

      // Verify cancellation response
      expect(mockAck).toHaveBeenCalled();
      expect(mockTaskManager.cancelTask).toHaveBeenCalledWith('task-cancel-123', 'U123456');
      expect(mockRespond).toHaveBeenCalledWith({
        text: 'Task `task-cancel-123` has been cancelled.',
        replace_original: true,
      });
    });
  });

  describe('API Integration Workflow', () => {
    it('should integrate with Orbit API during task creation', async () => {
      const command = {
        token: 'xoxb-test-token',
        team_id: 'T123456',
        team_domain: 'test-team',
        channel_id: 'C123456',
        channel_name: 'general',
        user_id: 'U123456',
        user_name: 'test-user',
        command: '/orbit-create',
        text: 'Create a new user profile page',
        response_url: 'https://hooks.slack.com/test',
        trigger_id: 'trigger-123',
      };

      const mockAck = vi.fn();
      const mockRespond = vi.fn();

      // Mock task creation that would call Orbit API
      const { mockTaskManager } = await import('../setup');
      mockTaskManager.createTask.mockResolvedValue({
        slack_task_id: 'task-api-123',
        status: 'pending',
        request: { prompt: 'Create a new user profile page', priority: 'medium' },
        created_at: new Date(),
        updated_at: new Date(),
      });

      // Get the create command handler
      const { mockSlackApp } = await import('../setup');
      const handler = mockSlackApp.command.mock.calls.find(
        ([cmd]) => cmd === '/orbit-create'
      )?.[1];

      // Execute the command
      await handler({ command, ack: mockAck, respond: mockRespond });

      // Verify task creation was initiated
      expect(mockTaskManager.createTask).toHaveBeenCalledWith('U123456', expect.objectContaining({
        prompt: 'Create a new user profile page',
      }));

      // Verify response was sent to user
      expect(mockRespond).toHaveBeenCalledWith({
        text: 'Task created successfully!',
        blocks: expect.arrayContaining([
          expect.objectContaining({
            type: 'section',
            text: expect.objectContaining({
              text: expect.stringContaining('task-api-123'),
            }),
          }),
        ]),
        response_type: 'in_channel',
      });
    });
  });

  describe('Health Check Workflow', () => {
    it('should perform comprehensive health checks', async () => {
      // Mock all services as healthy
      const { mockSlackApp, mockOrbitApiClient, mockDatabaseService, mockRedisService, mockTaskManager } = await import('../setup');
      
      mockSlackApp.client.auth.test.mockResolvedValue({
        ok: true,
        user: 'test-bot',
        team: 'test-team',
        bot_id: 'B123456',
      });

      mockOrbitApiClient.healthCheck.mockResolvedValue(true);
      mockDatabaseService.healthCheck.mockResolvedValue(true);
      mockRedisService.healthCheck.mockResolvedValue(true);
      mockTaskManager.healthCheck.mockResolvedValue({
        total_tasks: 15,
        active_tasks: 3,
        completed_tasks: 10,
        failed_tasks: 2,
      });

      // Perform health check
      const health = await slackBotService.healthCheck();

      // Verify all services are healthy
      expect(health).toEqual({
        slack: true,
        orbit: true,
        database: true,
        redis: true,
        tasks: {
          total_tasks: 15,
          active_tasks: 3,
          completed_tasks: 10,
          failed_tasks: 2,
        },
      });

      // Verify all health checks were called
      expect(mockSlackApp.client.auth.test).toHaveBeenCalled();
      expect(mockOrbitApiClient.healthCheck).toHaveBeenCalled();
      expect(mockDatabaseService.healthCheck).toHaveBeenCalled();
      expect(mockRedisService.healthCheck).toHaveBeenCalled();
      expect(mockTaskManager.healthCheck).toHaveBeenCalled();
    });

    it('should handle partial service failures in health check', async () => {
      // Mock some services as unhealthy
      const { mockSlackApp, mockOrbitApiClient, mockDatabaseService, mockRedisService, mockTaskManager } = await import('../setup');
      
      mockSlackApp.client.auth.test.mockRejectedValue(new Error('Slack API error'));
      mockOrbitApiClient.healthCheck.mockResolvedValue(false);
      mockDatabaseService.healthCheck.mockResolvedValue(true);
      mockRedisService.healthCheck.mockResolvedValue(false);
      mockTaskManager.healthCheck.mockResolvedValue({
        total_tasks: 0,
        active_tasks: 0,
        completed_tasks: 0,
        failed_tasks: 0,
      });

      // Perform health check
      const health = await slackBotService.healthCheck();

      // Verify partial failures are reported
      expect(health).toEqual({
        slack: false,
        orbit: false,
        database: true,
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
