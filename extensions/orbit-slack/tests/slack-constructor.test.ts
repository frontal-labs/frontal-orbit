import { beforeEach, describe, expect, it, vi } from 'vitest';

const slackState = vi.hoisted(() => {
  let messageHandler: ((args: { message: unknown }) => Promise<void>) | undefined;
  let commandHandler:
    | ((args: { command: { text: string; user_id: string; channel_id: string }; ack: (arg?: unknown) => Promise<void> | void }) => Promise<void>)
    | undefined;
  let actionHandler:
    | ((args: { action: unknown; ack: () => Promise<void> | void; body: unknown }) => Promise<void>)
    | undefined;
  let eventHandler: ((args: { event: unknown }) => Promise<void>) | undefined;

  const chatPostMessage = vi.fn();
  const chatUpdate = vi.fn();
  const appStart = vi.fn();
  const appStop = vi.fn();
  const isListening = vi.fn();

  class MockApp {
    public readonly client = {
      chat: {
        postMessage: chatPostMessage,
        update: chatUpdate,
      },
    };

    start = appStart;
    stop = appStop;
    isListening = isListening;

    constructor(_options: unknown) {}

    message(handler: typeof messageHandler): void {
      messageHandler = handler;
    }

    command(_name: string, handler: typeof commandHandler): void {
      commandHandler = handler;
    }

    action(_pattern: unknown, handler: typeof actionHandler): void {
      actionHandler = handler;
    }

    event(_pattern: unknown, handler: typeof eventHandler): void {
      eventHandler = handler;
    }
  }

  return {
    MockApp,
    chatPostMessage,
    chatUpdate,
    appStart,
    appStop,
    isListening,
    getMessageHandler: () => messageHandler,
    getCommandHandler: () => commandHandler,
    getActionHandler: () => actionHandler,
    getEventHandler: () => eventHandler,
    resetHandlers: () => {
      messageHandler = undefined;
      commandHandler = undefined;
      actionHandler = undefined;
      eventHandler = undefined;
    },
  };
});

const orbitApiState = vi.hoisted(() => {
  const createTask = vi.fn();
  const getOrphanPolicy = vi.fn();
  const sendConnectorEvent = vi.fn();
  const getEventsWebSocketUrl = vi.fn().mockReturnValue('ws://localhost:8787/v1/events/ws?source=slack');

  class MockOrbitApiClient {
    createTask = createTask;
    getOrphanPolicy = getOrphanPolicy;
    sendConnectorEvent = sendConnectorEvent;
    getEventsWebSocketUrl = getEventsWebSocketUrl;
  }

  return {
    MockOrbitApiClient,
    createTask,
    getOrphanPolicy,
    sendConnectorEvent,
    getEventsWebSocketUrl,
  };
});

const orbitEventsState = vi.hoisted(() => {
  let trackedHandler:
    | ((event: unknown, task: unknown) => Promise<void> | void)
    | undefined;

  const connect = vi.fn();
  const disconnect = vi.fn();
  const trackTask = vi.fn();
  const untrackTask = vi.fn();
  const onTrackedTaskEvent = vi.fn((handler: typeof trackedHandler) => {
    trackedHandler = handler;
    return () => {};
  });

  class MockOrbitEventsClient {
    connect = connect;
    disconnect = disconnect;
    trackTask = trackTask;
    untrackTask = untrackTask;
    onTrackedTaskEvent = onTrackedTaskEvent;

    constructor(_builder: unknown) {}
  }

  return {
    MockOrbitEventsClient,
    connect,
    disconnect,
    trackTask,
    untrackTask,
    onTrackedTaskEvent,
    getTrackedHandler: () => trackedHandler,
    resetTrackedHandler: () => {
      trackedHandler = undefined;
    },
  };
});

vi.mock('@slack/bolt', () => ({
  App: slackState.MockApp,
}));

vi.mock('../src/api-client', () => ({
  OrbitApiClient: orbitApiState.MockOrbitApiClient,
}));

vi.mock('../src/orbit-events', () => ({
  OrbitEventsClient: orbitEventsState.MockOrbitEventsClient,
}));

vi.mock('../src/config', () => ({
  config: {
    app: {
      nodeEnv: 'test',
      logLevel: 'error',
      port: 3000,
    },
    slack: {
      botToken: 'xoxb-test',
      appToken: 'xapp-test',
      signingSecret: 'secret',
    },
    orbit: {
      apiUrl: 'http://localhost:8787',
      timeout: 30_000,
    },
  },
}));

import { SlackInterface } from '../src/slack';

describe('SlackInterface constructor wiring', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    slackState.resetHandlers();
    orbitEventsState.resetTrackedHandler();
  });

  it('constructs the Slack app, Orbit clients, and tracked-event subscription', async () => {
    const handleSpy = vi
      .spyOn(SlackInterface.prototype as unknown as { handleOrbitTaskEvent: (...args: unknown[]) => Promise<void> }, 'handleOrbitTaskEvent')
      .mockResolvedValue(undefined);
    const slack = new SlackInterface();

    expect(orbitApiState.getEventsWebSocketUrl).not.toHaveBeenCalled();
    expect(orbitEventsState.onTrackedTaskEvent).toHaveBeenCalledTimes(1);
    expect(slackState.getMessageHandler()).toBeTypeOf('function');
    expect(slackState.getCommandHandler()).toBeTypeOf('function');
    expect(slackState.getActionHandler()).toBeTypeOf('function');
    expect(slackState.getEventHandler()).toBeTypeOf('function');

    await orbitEventsState.getTrackedHandler()?.({ event: 'lane.started' }, { taskId: 'task-123' });

    expect(handleSpy).toHaveBeenCalledWith({ event: 'lane.started' }, { taskId: 'task-123' });
  });

  it('ignores Slack message events without a user-facing prompt', async () => {
    new SlackInterface();

    await slackState.getMessageHandler()?.({
      message: {
        text: '',
        user: 'U123',
        channel: 'C123',
        ts: '1710000000.100',
      },
    });
    await slackState.getMessageHandler()?.({
      message: {
        text: 'hello',
        user: 'U123',
        channel: 'C123',
        ts: '1710000000.101',
        bot_id: 'B123',
      },
    });

    expect(orbitApiState.createTask).not.toHaveBeenCalled();
  });

  it('creates and registers a task from a Slack message event', async () => {
    const registerSpy = vi
      .spyOn(SlackInterface.prototype as unknown as { registerTask: (...args: unknown[]) => Promise<void> }, 'registerTask')
      .mockResolvedValue(undefined);
    orbitApiState.createTask.mockResolvedValue({
      task_id: 'task-123',
      status: 'running',
      message: 'created',
    });
    new SlackInterface();

    await slackState.getMessageHandler()?.({
      message: {
        text: 'Investigate flaky test',
        user: 'U123',
        channel: 'C123',
        ts: '1710000000.100',
      },
    });

    expect(orbitApiState.createTask).toHaveBeenCalledWith({
      prompt: 'Investigate flaky test',
      user_id: 'U123',
      channel_id: 'C123',
      thread_ts: '1710000000.100',
      source: 'slack',
    });
    expect(registerSpy).toHaveBeenCalledWith(
      expect.objectContaining({ task_id: 'task-123' }),
      {
        taskId: 'task-123',
        channelId: 'C123',
        threadTs: '1710000000.100',
        userId: 'U123',
      },
      'Investigate flaky test'
    );
  });

  it('acknowledges and returns policy previews for /ai policy orphans', async () => {
    const ack = vi.fn();
    orbitApiState.getOrphanPolicy.mockResolvedValue({
      default_policy: { source: 'default', approval_delay_secs: 60 },
      effective_policy: { source: 'default', approval_delay_secs: 60 },
      configured_rules: [],
    });
    new SlackInterface();

    await slackState.getCommandHandler()?.({
      command: {
        text: 'policy orphans',
        user_id: 'U123',
        channel_id: 'C123',
      },
      ack,
    });

    expect(orbitApiState.getOrphanPolicy).toHaveBeenCalledWith({});
    expect(ack).toHaveBeenCalledWith(
      expect.objectContaining({
        response_type: 'ephemeral',
      })
    );
    expect(orbitApiState.createTask).not.toHaveBeenCalled();
  });

  it('acknowledges policy preview failures with an ephemeral error response', async () => {
    const ack = vi.fn();
    orbitApiState.getOrphanPolicy.mockRejectedValue(new Error('policy unavailable'));
    new SlackInterface();

    await slackState.getCommandHandler()?.({
      command: {
        text: 'policy orphans repo=orbit/slack',
        user_id: 'U123',
        channel_id: 'C123',
      },
      ack,
    });

    expect(ack).toHaveBeenCalledWith({
      response_type: 'ephemeral',
      text: 'Failed to load orphan policy: policy unavailable',
    });
    expect(orbitApiState.createTask).not.toHaveBeenCalled();
  });

  it('creates a task from /ai commands after acknowledging', async () => {
    const ack = vi.fn();
    const registerSpy = vi
      .spyOn(SlackInterface.prototype as unknown as { registerTask: (...args: unknown[]) => Promise<void> }, 'registerTask')
      .mockResolvedValue(undefined);
    orbitApiState.createTask.mockResolvedValue({
      task_id: 'task-456',
      status: 'running',
      message: 'created',
    });
    new SlackInterface();

    await slackState.getCommandHandler()?.({
      command: {
        text: 'Fix the flaky test',
        user_id: 'U123',
        channel_id: 'C123',
      },
      ack,
    });

    expect(ack).toHaveBeenCalledWith();
    expect(orbitApiState.createTask).toHaveBeenCalledWith({
      prompt: 'Fix the flaky test',
      user_id: 'U123',
      channel_id: 'C123',
      source: 'slack',
    });
    expect(registerSpy).toHaveBeenCalledWith(
      expect.objectContaining({ task_id: 'task-456' }),
      {
        taskId: 'task-456',
        channelId: 'C123',
        userId: 'U123',
      },
      'Fix the flaky test'
    );
  });

  it('acknowledges actions and forwards them to the Slack action handler', async () => {
    const ack = vi.fn();
    const handleSpy = vi
      .spyOn(SlackInterface.prototype as unknown as { handleSlackAction: (...args: unknown[]) => Promise<void> }, 'handleSlackAction')
      .mockResolvedValue(undefined);
    new SlackInterface();

    const body = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.100' },
    };
    const action = { action_id: 'orphaned_hosted_agent.retry', value: 'task-123' };

    await slackState.getActionHandler()?.({ action, ack, body });

    expect(ack).toHaveBeenCalledWith();
    expect(handleSpy).toHaveBeenCalledWith(action, body);
  });

  it('forwards Slack events to the connector event endpoint', async () => {
    new SlackInterface();

    await slackState.getEventHandler()?.({
      event: {
        type: 'reaction_added',
        user: 'U123',
        reaction: 'eyes',
      },
    });

    expect(orbitApiState.sendConnectorEvent).toHaveBeenCalledWith('slack', {
      type: 'reaction_added',
      userId: 'U123',
      data: {
        type: 'reaction_added',
        user: 'U123',
        reaction: 'eyes',
      },
    });
  });

  it('forwards Slack events without a user as connector events with an empty user id', async () => {
    new SlackInterface();

    await slackState.getEventHandler()?.({
      event: {
        type: 'member_joined_channel',
        channel: 'C123',
      },
    });

    expect(orbitApiState.sendConnectorEvent).toHaveBeenCalledWith('slack', {
      type: 'member_joined_channel',
      userId: '',
      data: {
        type: 'member_joined_channel',
        channel: 'C123',
      },
    });
  });
});
