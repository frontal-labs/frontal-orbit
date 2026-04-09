import { describe, expect, it } from 'vitest';
import type {
  SlackBlock,
  SlackChannel,
  SlackCommand,
  SlackConversationContext,
  SlackInteraction,
  SlackMessage,
  SlackMessageOptions,
  SlackUser,
} from '../../src/types';

describe('Slack Types', () => {
  describe('Type Definitions', () => {
    it('should have proper SlackUser interface structure', () => {
      const user: SlackUser = {
        id: 'U123456',
        name: 'test-user',
        real_name: 'Test User',
        email: 'test@example.com',
      };

      expect(user.id).toBe('U123456');
      expect(user.name).toBe('test-user');
      expect(user.real_name).toBe('Test User');
      expect(user.email).toBe('test@example.com');
    });

    it('should allow optional email in SlackUser', () => {
      const user: SlackUser = {
        id: 'U123456',
        name: 'test-user',
        real_name: 'Test User',
      };

      expect(user.email).toBeUndefined();
    });

    it('should have proper SlackChannel interface structure', () => {
      const channel: SlackChannel = {
        id: 'C123456',
        name: 'general',
        is_private: false,
      };

      expect(channel.id).toBe('C123456');
      expect(channel.name).toBe('general');
      expect(channel.is_private).toBe(false);
    });

    it('should have proper SlackMessage interface structure', () => {
      const message: SlackMessage = {
        user: 'U123456',
        channel: 'C123456',
        text: 'Hello world',
        ts: '1640995200.000000',
        thread_ts: '1640995200.000000',
        team: 'T123456',
      };

      expect(message.user).toBe('U123456');
      expect(message.channel).toBe('C123456');
      expect(message.text).toBe('Hello world');
      expect(message.ts).toBe('1640995200.000000');
      expect(message.thread_ts).toBe('1640995200.000000');
      expect(message.team).toBe('T123456');
    });

    it('should allow optional fields in SlackMessage', () => {
      const message: SlackMessage = {
        user: 'U123456',
        channel: 'C123456',
        text: 'Hello world',
        ts: '1640995200.000000',
      };

      expect(message.thread_ts).toBeUndefined();
      expect(message.team).toBeUndefined();
    });

    it('should have proper SlackCommand interface structure', () => {
      const command: SlackCommand = {
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
      };

      expect(command.token).toBe('test-token');
      expect(command.team_id).toBe('T123456');
      expect(command.team_domain).toBe('test-team');
      expect(command.channel_id).toBe('C123456');
      expect(command.channel_name).toBe('general');
      expect(command.user_id).toBe('U123456');
      expect(command.user_name).toBe('test-user');
      expect(command.command).toBe('/orbit-create');
      expect(command.text).toBe('Test task description');
      expect(command.response_url).toBe('https://hooks.slack.com/test');
      expect(command.trigger_id).toBe('trigger-123');
    });

    it('should have proper SlackInteraction interface structure', () => {
      const interaction: SlackInteraction = {
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
      };

      expect(interaction.type).toBe('interactive_message');
      expect(interaction.token).toBe('test-token');
      expect(interaction.action_ts).toBe('1640995200.000000');
      expect(interaction.team.id).toBe('T123456');
      expect(interaction.team.domain).toBe('test-team');
      expect(interaction.user.id).toBe('U123456');
      expect(interaction.user.name).toBe('test-user');
      expect(interaction.channel.id).toBe('C123456');
      expect(interaction.channel.name).toBe('general');
      expect(interaction.actions).toHaveLength(1);
      expect(interaction.actions[0].name).toBe('task_cancel');
      expect(interaction.actions[0].type).toBe('button');
      expect(interaction.actions[0].value).toBe('task-123');
    });

    it('should have proper SlackBlock interface structure', () => {
      const block: SlackBlock = {
        type: 'section',
        text: {
          type: 'mrkdwn',
          text: '*Hello World*',
          emoji: true,
        },
        accessory: {
          type: 'button',
          text: {
            type: 'plain_text',
            text: 'Click me',
          },
          value: 'button-123',
        },
      };

      expect(block.type).toBe('section');
      expect(block.text?.type).toBe('mrkdwn');
      expect(block.text?.text).toBe('*Hello World*');
      expect(block.text?.emoji).toBe(true);
      expect(block.accessory).toBeDefined();
    });

    it('should allow optional fields in SlackBlock', () => {
      const block: SlackBlock = {
        type: 'section',
      };

      expect(block.text).toBeUndefined();
      expect(block.accessory).toBeUndefined();
      expect(block.elements).toBeUndefined();
    });

    it('should have proper SlackMessageOptions interface structure', () => {
      const options: SlackMessageOptions = {
        channel: 'C123456',
        text: 'Hello world',
        blocks: [
          {
            type: 'section',
            text: {
              type: 'mrkdwn',
              text: '*Hello World*',
            },
          },
        ],
        thread_ts: '1640995200.000000',
        attachments: [
          {
            color: 'good',
            text: 'Success!',
          },
        ],
      };

      expect(options.channel).toBe('C123456');
      expect(options.text).toBe('Hello world');
      expect(options.blocks).toHaveLength(1);
      expect(options.thread_ts).toBe('1640995200.000000');
      expect(options.attachments).toHaveLength(1);
    });

    it('should allow optional fields in SlackMessageOptions', () => {
      const options: SlackMessageOptions = {
        channel: 'C123456',
      };

      expect(options.text).toBeUndefined();
      expect(options.blocks).toBeUndefined();
      expect(options.thread_ts).toBeUndefined();
      expect(options.attachments).toBeUndefined();
    });

    it('should have proper SlackConversationContext interface structure', () => {
      const context: SlackConversationContext = {
        channel_id: 'C123456',
        thread_ts: '1640995200.000000',
        user_id: 'U123456',
        context: {
          current_task: 'task-123',
          repository: 'test-org/test-repo',
          branch: 'main',
          last_command: '/orbit-create',
          preferences: {
            model: 'claude-3-sonnet',
            provider: 'anthropic',
          },
        },
        created_at: new Date('2024-01-01T00:00:00Z'),
        updated_at: new Date('2024-01-01T01:00:00Z'),
      };

      expect(context.channel_id).toBe('C123456');
      expect(context.thread_ts).toBe('1640995200.000000');
      expect(context.user_id).toBe('U123456');
      expect(context.context.current_task).toBe('task-123');
      expect(context.context.repository).toBe('test-org/test-repo');
      expect(context.context.branch).toBe('main');
      expect(context.context.last_command).toBe('/orbit-create');
      expect(context.context.preferences.model).toBe('claude-3-sonnet');
      expect(context.context.preferences.provider).toBe('anthropic');
      expect(context.created_at).toEqual(new Date('2024-01-01T00:00:00Z'));
      expect(context.updated_at).toEqual(new Date('2024-01-01T01:00:00Z'));
    });

    it('should allow optional fields in SlackConversationContext', () => {
      const context: SlackConversationContext = {
        channel_id: 'C123456',
        user_id: 'U123456',
        context: {
          preferences: {},
        },
        created_at: new Date(),
        updated_at: new Date(),
      };

      expect(context.thread_ts).toBeUndefined();
      expect(context.context.current_task).toBeUndefined();
      expect(context.context.repository).toBeUndefined();
      expect(context.context.branch).toBeUndefined();
      expect(context.context.last_command).toBeUndefined();
    });
  });

  describe('Type Safety', () => {
    it('should enforce required fields', () => {
      // These should compile without errors
      const user: import('@/types/slack-types').SlackUser = {
        id: 'U123456',
        name: 'test-user',
        real_name: 'Test User',
      };

      const channel: import('@/types/slack-types').SlackChannel = {
        id: 'C123456',
        name: 'general',
        is_private: false,
      };

      const message: import('@/types/slack-types').SlackMessage = {
        user: 'U123456',
        channel: 'C123456',
        text: 'Hello',
        ts: '1640995200.000000',
      };

      expect(user.id).toBeDefined();
      expect(channel.id).toBeDefined();
      expect(message.user).toBeDefined();
    });

    it('should allow proper type extensions', () => {
      // Test that types can be extended with additional properties
      const extendedBlock: import('@/types/slack-types').SlackBlock & {
        custom_field?: string;
      } = {
        type: 'section',
        custom_field: 'custom value',
      };

      expect(extendedBlock.custom_field).toBe('custom value');
    });
  });

  describe('Runtime Type Validation', () => {
    it('should validate SlackUser structure at runtime', () => {
      const data = {
        id: 'U123456',
        name: 'test-user',
        real_name: 'Test User',
        email: 'test@example.com',
      };

      // Type assertion for testing
      const user = data as import('@/types/slack-types').SlackUser;

      expect(typeof user.id).toBe('string');
      expect(typeof user.name).toBe('string');
      expect(typeof user.real_name).toBe('string');
      expect(user.email === undefined || typeof user.email === 'string').toBe(true);
    });

    it('should validate SlackCommand structure at runtime', () => {
      const data = {
        token: 'test-token',
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

      const command = data as import('@/types/slack-types').SlackCommand;

      expect(typeof command.token).toBe('string');
      expect(typeof command.team_id).toBe('string');
      expect(typeof command.team_domain).toBe('string');
      expect(typeof command.channel_id).toBe('string');
      expect(typeof command.channel_name).toBe('string');
      expect(typeof command.user_id).toBe('string');
      expect(typeof command.user_name).toBe('string');
      expect(typeof command.command).toBe('string');
      expect(typeof command.text).toBe('string');
      expect(typeof command.response_url).toBe('string');
      expect(typeof command.trigger_id).toBe('string');
    });

    it('should validate SlackInteraction structure at runtime', () => {
      const data = {
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
      };

      const interaction = data as import('@/types/slack-types').SlackInteraction;

      expect(typeof interaction.type).toBe('string');
      expect(typeof interaction.token).toBe('string');
      expect(typeof interaction.action_ts).toBe('string');
      expect(typeof interaction.team.id).toBe('string');
      expect(typeof interaction.team.domain).toBe('string');
      expect(typeof interaction.user.id).toBe('string');
      expect(typeof interaction.user.name).toBe('string');
      expect(typeof interaction.channel.id).toBe('string');
      expect(typeof interaction.channel.name).toBe('string');
      expect(Array.isArray(interaction.actions)).toBe(true);
      expect(typeof interaction.actions[0].name).toBe('string');
      expect(typeof interaction.actions[0].type).toBe('string');
      expect(typeof interaction.actions[0].value).toBe('string');
    });
  });

  describe('Edge Cases', () => {
    it('should handle empty strings in optional fields', () => {
      const user: import('@/types/slack-types').SlackUser = {
        id: 'U123456',
        name: 'test-user',
        real_name: 'Test User',
        email: '', // Empty string is valid
      };

      expect(user.email).toBe('');
    });

    it('should handle arrays with empty blocks', () => {
      const options: import('@/types/slack-types').SlackMessageOptions = {
        channel: 'C123456',
        blocks: [], // Empty array is valid
      };

      expect(options.blocks).toHaveLength(0);
    });

    it('should handle empty context object', () => {
      const context: import('@/types/slack-types').SlackConversationContext = {
        channel_id: 'C123456',
        user_id: 'U123456',
        context: {}, // Empty context is valid
        created_at: new Date(),
        updated_at: new Date(),
      };

      expect(Object.keys(context.context)).toHaveLength(0);
    });
  });
});
