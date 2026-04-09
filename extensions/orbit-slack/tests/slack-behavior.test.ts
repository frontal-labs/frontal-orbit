import { beforeEach, describe, expect, it, vi } from 'vitest';
import { logger } from '../src/log';
import { SlackInterface } from '../src/slack';
import type {
  OrbitCreateTaskResponse,
  OrbitEventEnvelope,
  OrbitTask,
  OrbitTrackedTask,
  OrbitUpdateTaskContextRequest,
  SlackBody,
} from '../src/types';

type TestableSlackInterface = SlackInterface & {
  connect(): Promise<void>;
  disconnect(): Promise<void>;
  healthCheck(): Promise<{ slack: boolean; orbit: boolean }>;
  registerTask(
    task: OrbitCreateTaskResponse,
    trackedTask: OrbitTrackedTask,
    prompt: string
  ): Promise<void>;
  handleSlackAction(action: { action_id: string; value?: string }, body: SlackBody): Promise<void>;
  handleOrphanApprovalAction(
    action: { action_id: string; value?: string },
    body: SlackBody
  ): Promise<void>;
  handleOrbitTaskEvent(event: OrbitEventEnvelope, task?: OrbitTrackedTask): Promise<void>;
  resolveTrackedTaskForEvent(
    event: OrbitEventEnvelope,
    task?: OrbitTrackedTask
  ): Promise<OrbitTrackedTask | undefined>;
  hydrateTrackedTaskFromEvent(
    task: OrbitTrackedTask,
    event: OrbitEventEnvelope
  ): OrbitTrackedTask;
  upsertTrackedTask(task: OrbitTrackedTask): void;
  syncTrackedTasksFromOrbit(): Promise<void>;
  trackedTasks: Map<string, OrbitTrackedTask>;
  approvalMessageTsByTask: Map<string, string>;
  approvalInFlight: Set<string>;
  approvalResolved: Set<string>;
};

function createTestSlackInterface() {
  const slack = Object.create(SlackInterface.prototype) as TestableSlackInterface;
  const postMessage = vi.fn();
  const updateMessage = vi.fn();
  const updateTaskContext = vi.fn();
  const getTask = vi.fn();
  const listTasks = vi.fn();
  const resolveTaskApproval = vi.fn();
  const healthCheck = vi.fn();
  const sendConnectorInteraction = vi.fn();
  const appStart = vi.fn();
  const appStop = vi.fn();
  const isListening = vi.fn();
  const connectEvents = vi.fn();
  const disconnectEvents = vi.fn();
  const trackTask = vi.fn();
  const untrackTask = vi.fn();

  Object.assign(slack, {
    app: {
      start: appStart,
      stop: appStop,
      isListening,
      client: {
        chat: {
          postMessage,
          update: updateMessage,
        },
      },
    },
    orbitApi: {
      updateTaskContext,
      getTask,
      listTasks,
      resolveTaskApproval,
      healthCheck,
      sendConnectorInteraction,
    },
    orbitEvents: {
      connect: connectEvents,
      disconnect: disconnectEvents,
      trackTask,
      untrackTask,
    },
    trackedTasks: new Map<string, OrbitTrackedTask>(),
    approvalMessageTsByTask: new Map<string, string>(),
    approvalInFlight: new Set<string>(),
    approvalResolved: new Set<string>(),
  });

  return {
    slack,
    postMessage,
    updateMessage,
    updateTaskContext,
    getTask,
    listTasks,
    resolveTaskApproval,
    healthCheck,
    sendConnectorInteraction,
    appStart,
    appStop,
    isListening,
    connectEvents,
    disconnectEvents,
    trackTask,
    untrackTask,
  };
}

function createTrackedTask(overrides: Partial<OrbitTrackedTask> = {}): OrbitTrackedTask {
  return {
    taskId: 'task-123',
    channelId: 'C123',
    threadTs: '1710000000.100',
    userId: 'U123',
    ...overrides,
  };
}

function createTaskResponse(
  overrides: Partial<OrbitCreateTaskResponse> = {}
): OrbitCreateTaskResponse {
  return {
    task_id: 'task-123',
    status: 'running',
    message: 'created',
    ...overrides,
  };
}

function createTaskSnapshot(overrides: Partial<OrbitTask> = {}): OrbitTask {
  return {
    task_id: 'task-123',
    prompt: 'Investigate flaky test',
    status: 'running',
    created_at: 1,
    updated_at: 2,
    ...overrides,
  };
}

function createEvent(overrides: Partial<OrbitEventEnvelope> = {}): OrbitEventEnvelope {
  return {
    event_id: 'evt-123',
    topic: 'approval',
    event: 'approval.requested',
    status: 'waiting',
    emittedAt: '2026-04-09T10:00:00Z',
    task_id: 'task-123',
    payload: {
      approval_kind: 'orphaned_hosted_agent',
      reason: 'Hosted lane lost its executor',
      channel_id: 'C123',
      thread_ts: '1710000000.100',
      user_id: 'U123',
      repository: 'orbit/slack',
    },
    ...overrides,
  };
}

describe('SlackInterface behavior', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('registers a created task, persists thread context, and tracks it for Orbit events', async () => {
    const { slack, postMessage, updateTaskContext, trackTask } = createTestSlackInterface();
    const task = createTaskResponse({
      plan_kind: 'implementer',
      worker_status: 'ready',
      worker_id: 'worker-123',
    });
    const trackedTask = createTrackedTask({ threadTs: undefined });
    postMessage.mockResolvedValue({ ts: '1710000000.200' });
    updateTaskContext.mockResolvedValue({});

    await slack.registerTask(task, trackedTask, 'Investigate flaky test');

    expect(postMessage).toHaveBeenCalledWith({
      channel: 'C123',
      text: expect.stringContaining('Task created: task-123'),
      thread_ts: undefined,
    });
    expect(updateTaskContext).toHaveBeenCalledWith({
      taskId: 'task-123',
      source: 'slack',
      user_id: 'U123',
      channel_id: 'C123',
      thread_ts: '1710000000.200',
    } satisfies OrbitUpdateTaskContextRequest);
    expect(trackTask).toHaveBeenCalledWith({
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.200',
      userId: 'U123',
    });
    expect(slack.trackedTasks.get('task-123')).toEqual({
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.200',
      userId: 'U123',
    });
  });

  it('connects by syncing tasks, starting Slack, and connecting Orbit events', async () => {
    const infoSpy = vi.spyOn(logger, 'info').mockImplementation(() => {});
    const { slack, listTasks, appStart, connectEvents } = createTestSlackInterface();
    listTasks.mockResolvedValue([]);
    appStart.mockResolvedValue(undefined);
    connectEvents.mockResolvedValue(undefined);

    await slack.connect();

    expect(listTasks).toHaveBeenCalledWith({
      source: 'slack',
      status: 'pending,running',
    });
    expect(appStart).toHaveBeenCalled();
    expect(connectEvents).toHaveBeenCalled();
    expect(infoSpy).toHaveBeenCalledWith('Slack WebSocket interface connected');
  });

  it('rethrows connect failures after logging them', async () => {
    const errorSpy = vi.spyOn(logger, 'error').mockImplementation(() => {});
    const { slack, listTasks, appStart, connectEvents } = createTestSlackInterface();
    listTasks.mockResolvedValue([]);
    appStart.mockResolvedValue(undefined);
    connectEvents.mockRejectedValue(new Error('ws down'));

    await expect(slack.connect()).rejects.toThrow('ws down');
    expect(errorSpy).toHaveBeenCalledWith('Failed to connect to Slack', expect.any(Error));
  });

  it('disconnects Orbit events before stopping Slack', async () => {
    const infoSpy = vi.spyOn(logger, 'info').mockImplementation(() => {});
    const { slack, disconnectEvents, appStop } = createTestSlackInterface();
    disconnectEvents.mockResolvedValue(undefined);
    appStop.mockResolvedValue(undefined);

    await slack.disconnect();

    expect(disconnectEvents).toHaveBeenCalled();
    expect(appStop).toHaveBeenCalled();
    expect(infoSpy).toHaveBeenCalledWith('Slack WebSocket interface disconnected');
  });

  it('rethrows disconnect failures after logging them', async () => {
    const errorSpy = vi.spyOn(logger, 'error').mockImplementation(() => {});
    const { slack, disconnectEvents, appStop } = createTestSlackInterface();
    disconnectEvents.mockRejectedValue(new Error('disconnect failed'));

    await expect(slack.disconnect()).rejects.toThrow('disconnect failed');
    expect(appStop).not.toHaveBeenCalled();
    expect(errorSpy).toHaveBeenCalledWith('Failed to disconnect from Slack', expect.any(Error));
  });

  it('reports Slack and Orbit health from the live clients', async () => {
    const { slack, isListening, healthCheck } = createTestSlackInterface();
    isListening.mockReturnValue(true);
    healthCheck.mockResolvedValue(true);

    await expect(slack.healthCheck()).resolves.toEqual({
      slack: true,
      orbit: true,
    });
  });

  it('treats Slack as disconnected when the Bolt client does not expose isListening', async () => {
    const { slack, healthCheck } = createTestSlackInterface();
    delete (slack.app as { isListening?: () => boolean }).isListening;
    healthCheck.mockResolvedValue(true);

    await expect(slack.healthCheck()).resolves.toEqual({
      slack: false,
      orbit: true,
    });
  });

  it('returns a degraded health response when health checks throw', async () => {
    const errorSpy = vi.spyOn(logger, 'error').mockImplementation(() => {});
    const { slack, isListening } = createTestSlackInterface();
    isListening.mockImplementation(() => {
      throw new Error('listener unavailable');
    });

    await expect(slack.healthCheck()).resolves.toEqual({
      slack: false,
      orbit: false,
    });
    expect(errorSpy).toHaveBeenCalledWith('Health check failed', expect.any(Error));
  });

  it('still tracks a task when thread-anchor persistence back to Orbit fails', async () => {
    const warnSpy = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const { slack, postMessage, updateTaskContext, trackTask } = createTestSlackInterface();
    postMessage.mockResolvedValue({ ts: '1710000000.250' });
    updateTaskContext.mockRejectedValue(new Error('orbit unavailable'));

    await slack.registerTask(
      createTaskResponse(),
      createTrackedTask({ threadTs: undefined }),
      'Investigate flaky test'
    );

    expect(trackTask).toHaveBeenCalledWith({
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.250',
      userId: 'U123',
    });
    expect(slack.trackedTasks.get('task-123')?.threadTs).toBe('1710000000.250');
    expect(warnSpy).toHaveBeenCalledWith(
      'Failed to persist Slack task thread anchor to Orbit',
      expect.objectContaining({
        taskId: 'task-123',
        error: 'orbit unavailable',
      })
    );
  });

  it('routes generic connector actions through the hosted connector interaction endpoint', async () => {
    const { slack, sendConnectorInteraction, updateMessage } = createTestSlackInterface();
    const body: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.500' },
    };
    sendConnectorInteraction.mockResolvedValue({
      blocks: [
        {
          type: 'section',
          text: { type: 'mrkdwn', text: '*Updated*' },
        },
      ],
    });

    await slack.handleSlackAction(
      {
        action_id: 'connector.action',
        value: 'task-123',
      },
      body
    );

    expect(sendConnectorInteraction).toHaveBeenCalledWith('slack', {
      action: 'connector.action',
      value: 'task-123',
      userId: 'U123',
      context: body,
    });
    expect(updateMessage).toHaveBeenCalledWith({
      channel: 'C123',
      ts: '1710000000.500',
      blocks: expect.any(Array),
    });
  });

  it('routes orphan approval actions through the dedicated approval handler', async () => {
    const { slack, sendConnectorInteraction } = createTestSlackInterface();
    const approvalSpy = vi
      .spyOn(slack, 'handleOrphanApprovalAction')
      .mockResolvedValue(undefined);
    const body: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.500' },
    };

    await slack.handleSlackAction(
      {
        action_id: 'orphaned_hosted_agent.retry',
        value: 'task-123',
      },
      body
    );

    expect(approvalSpy).toHaveBeenCalledWith(
      {
        action_id: 'orphaned_hosted_agent.retry',
        value: 'task-123',
      },
      body
    );
    expect(sendConnectorInteraction).not.toHaveBeenCalled();
  });

  it('short-circuits orphan approvals that were already resolved', async () => {
    const { slack, updateMessage } = createTestSlackInterface();
    const body: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.510' },
    };
    slack.approvalResolved.add('task-123');

    await slack.handleOrphanApprovalAction(
      {
        action_id: 'orphaned_hosted_agent.retry',
        value: 'task-123',
      },
      body
    );

    expect(updateMessage).toHaveBeenCalledWith({
      channel: 'C123',
      ts: '1710000000.510',
      text: 'Approval for task task-123 was already resolved.',
      blocks: expect.any(Array),
    });
    expect(slack.approvalInFlight.has('task-123')).toBe(false);
  });

  it('short-circuits orphan approvals already being processed', async () => {
    const { slack, updateMessage } = createTestSlackInterface();
    const body: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.520' },
    };
    slack.approvalInFlight.add('task-123');

    await slack.handleOrphanApprovalAction(
      {
        action_id: 'orphaned_hosted_agent.cancel',
        value: 'task-123',
      },
      body
    );

    expect(updateMessage).toHaveBeenCalledWith({
      channel: 'C123',
      ts: '1710000000.520',
      text: 'Approval for task task-123 is already being processed.',
      blocks: expect.any(Array),
    });
  });

  it('ignores orphan approval actions that do not carry a task id', async () => {
    const { slack, updateMessage, resolveTaskApproval } = createTestSlackInterface();
    const body: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.530' },
    };

    await slack.handleOrphanApprovalAction(
      {
        action_id: 'orphaned_hosted_agent.retry',
      },
      body
    );

    expect(updateMessage).not.toHaveBeenCalled();
    expect(resolveTaskApproval).not.toHaveBeenCalled();
  });

  it('resolves orphan approvals through Orbit and updates the approval message twice', async () => {
    const { slack, updateMessage, resolveTaskApproval } = createTestSlackInterface();
    const body: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.540' },
    };
    resolveTaskApproval.mockResolvedValue(
      createTaskSnapshot({
        status: 'running',
        worker_status: 'ready_for_prompt',
        worker_id: 'worker-123',
      })
    );

    await slack.handleOrphanApprovalAction(
      {
        action_id: 'orphaned_hosted_agent.retry',
        value: 'task-123',
      },
      body
    );

    expect(resolveTaskApproval).toHaveBeenCalledWith({
      taskId: 'task-123',
      approvalKind: 'orphaned_hosted_agent',
      action: 'retry',
      resolvedBy: 'U123',
      reason: 'resolved from Slack approval action',
    });
    expect(updateMessage).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        channel: 'C123',
        ts: '1710000000.540',
        text: 'Processing approval for task task-123: retry.',
      })
    );
    expect(updateMessage).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        channel: 'C123',
        ts: '1710000000.540',
        text: 'Approval resolved for task task-123: retry.',
      })
    );
    expect(slack.approvalInFlight.has('task-123')).toBe(false);
    expect(slack.approvalResolved.has('task-123')).toBe(true);
  });

  it('updates the approval message with an error state when resolution fails', async () => {
    const { slack, updateMessage, resolveTaskApproval } = createTestSlackInterface();
    const body: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.550' },
    };
    resolveTaskApproval.mockRejectedValue(new Error('provider down'));

    await expect(
      slack.handleOrphanApprovalAction(
        {
          action_id: 'orphaned_hosted_agent.cancel',
          value: 'task-123',
        },
        body
      )
    ).rejects.toThrow('provider down');

    expect(updateMessage).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        text: 'Processing approval for task task-123: cancel.',
      })
    );
    expect(updateMessage).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        text: 'Approval failed for task task-123.',
      })
    );
    expect(slack.approvalInFlight.has('task-123')).toBe(false);
  });

  it('resolves orphan approvals, updates the message twice, and marks the approval resolved', async () => {
    const { slack, updateMessage, resolveTaskApproval } = createTestSlackInterface();
    const body: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.530' },
    };
    resolveTaskApproval.mockResolvedValue(
      createTaskSnapshot({
        worker_status: 'ready_for_prompt',
        worker_id: 'worker-123',
      })
    );

    await slack.handleOrphanApprovalAction(
      {
        action_id: 'orphaned_hosted_agent.retry',
        value: 'task-123',
      },
      body
    );

    expect(resolveTaskApproval).toHaveBeenCalledWith({
      taskId: 'task-123',
      approvalKind: 'orphaned_hosted_agent',
      action: 'retry',
      resolvedBy: 'U123',
      reason: 'resolved from Slack approval action',
    });
    expect(updateMessage).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        channel: 'C123',
        ts: '1710000000.530',
        text: 'Processing approval for task task-123: retry.',
        blocks: expect.any(Array),
      })
    );
    expect(updateMessage).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        channel: 'C123',
        ts: '1710000000.530',
        text: 'Approval resolved for task task-123: retry.',
        blocks: expect.any(Array),
      })
    );
    expect(slack.approvalInFlight.has('task-123')).toBe(false);
    expect(slack.approvalResolved.has('task-123')).toBe(true);
    expect(slack.approvalMessageTsByTask.get('task-123')).toBe('1710000000.530');
  });

  it('shows an approval error state, clears in-flight state, and rethrows when resolution fails', async () => {
    const { slack, updateMessage, resolveTaskApproval } = createTestSlackInterface();
    const body: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.540' },
    };
    const failure = new Error('approval backend unavailable');
    resolveTaskApproval.mockRejectedValue(failure);

    await expect(
      slack.handleOrphanApprovalAction(
        {
          action_id: 'orphaned_hosted_agent.cancel',
          value: 'task-123',
        },
        body
      )
    ).rejects.toBe(failure);

    expect(updateMessage).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        channel: 'C123',
        ts: '1710000000.540',
        text: 'Processing approval for task task-123: cancel.',
        blocks: expect.any(Array),
      })
    );
    expect(updateMessage).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        channel: 'C123',
        ts: '1710000000.540',
        text: 'Approval failed for task task-123.',
        blocks: expect.any(Array),
      })
    );
    expect(slack.approvalInFlight.has('task-123')).toBe(false);
    expect(slack.approvalResolved.has('task-123')).toBe(false);
    expect(slack.approvalMessageTsByTask.get('task-123')).toBe('1710000000.540');
  });

  it('leaves the approval marked in-flight when the initial Slack approval update fails', async () => {
    const { slack, updateMessage, resolveTaskApproval } = createTestSlackInterface();
    const body: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.550' },
    };
    updateMessage.mockRejectedValue(new Error('slack update failed'));

    await expect(
      slack.handleOrphanApprovalAction(
        {
          action_id: 'orphaned_hosted_agent.retry',
          value: 'task-123',
        },
        body
      )
    ).rejects.toThrow('slack update failed');

    expect(resolveTaskApproval).not.toHaveBeenCalled();
    expect(slack.approvalInFlight.has('task-123')).toBe(true);
    expect(slack.approvalResolved.has('task-123')).toBe(false);
    expect(slack.approvalMessageTsByTask.get('task-123')).toBe('1710000000.550');
  });

  it('posts approval-request events into the task thread and persists the approval message ts', async () => {
    const { slack, postMessage, updateTaskContext } = createTestSlackInterface();
    const trackedTask = createTrackedTask();
    postMessage.mockResolvedValue({ ts: '1710000000.300' });
    updateTaskContext.mockResolvedValue({});

    await slack.handleOrbitTaskEvent(createEvent(), trackedTask);

    expect(postMessage).toHaveBeenCalledWith({
      channel: 'C123',
      text: expect.stringContaining('needs approval'),
      blocks: expect.any(Array),
      thread_ts: '1710000000.100',
    });
    expect(slack.approvalMessageTsByTask.get('task-123')).toBe('1710000000.300');
    expect(updateTaskContext).toHaveBeenCalledWith({
      taskId: 'task-123',
      source: 'slack',
      user_id: 'U123',
      channel_id: 'C123',
      thread_ts: '1710000000.100',
      approval_message_ts: '1710000000.300',
    });
  });

  it('returns without posting when an Orbit event cannot be resolved to a tracked task', async () => {
    const { slack, postMessage } = createTestSlackInterface();
    const resolveSpy = vi
      .spyOn(slack, 'resolveTrackedTaskForEvent')
      .mockResolvedValue(undefined);

    await slack.handleOrbitTaskEvent(createEvent());

    expect(resolveSpy).toHaveBeenCalled();
    expect(postMessage).not.toHaveBeenCalled();
  });

  it('suppresses repeated approval-request events after the approval is already resolved', async () => {
    const { slack, postMessage } = createTestSlackInterface();
    const trackedTask = createTrackedTask();
    slack.approvalResolved.add('task-123');

    await slack.handleOrbitTaskEvent(createEvent(), trackedTask);

    expect(postMessage).not.toHaveBeenCalled();
  });

  it('keeps the posted approval message even when approval-message persistence back to Orbit fails', async () => {
    const warnSpy = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const { slack, postMessage, updateTaskContext } = createTestSlackInterface();
    const trackedTask = createTrackedTask();
    postMessage.mockResolvedValue({ ts: '1710000000.301' });
    updateTaskContext.mockRejectedValue(new Error('persist failed'));

    await slack.handleOrbitTaskEvent(createEvent(), trackedTask);

    expect(postMessage).toHaveBeenCalled();
    expect(slack.approvalMessageTsByTask.get('task-123')).toBe('1710000000.301');
    expect(warnSpy).toHaveBeenCalledWith(
      'Failed to persist Slack approval message linkage to Orbit',
      expect.objectContaining({
        taskId: 'task-123',
        error: 'persist failed',
      })
    );
  });

  it('updates an existing approval message when an approval is resolved', async () => {
    const { slack, postMessage, updateMessage } = createTestSlackInterface();
    const trackedTask = createTrackedTask();
    slack.approvalMessageTsByTask.set('task-123', '1710000000.300');
    slack.approvalInFlight.add('task-123');

    await slack.handleOrbitTaskEvent(
      createEvent({
        event: 'approval.resolved',
        status: 'running',
        payload: {
          action: 'retry',
          worker_status: 'ready_for_prompt',
          worker_id: 'worker-123',
          channel_id: 'C123',
          repository: 'orbit/slack',
        },
      }),
      trackedTask
    );

    expect(updateMessage).toHaveBeenCalledWith({
      channel: 'C123',
      ts: '1710000000.300',
      text: expect.stringContaining('Approval resolved'),
      blocks: expect.any(Array),
    });
    expect(postMessage).not.toHaveBeenCalled();
    expect(slack.approvalInFlight.has('task-123')).toBe(false);
    expect(slack.approvalResolved.has('task-123')).toBe(true);
  });

  it('cleans up tracked task state after a terminal approval-resolved update succeeds', async () => {
    const { slack, postMessage, updateMessage, untrackTask } = createTestSlackInterface();
    const trackedTask = createTrackedTask();
    slack.trackedTasks.set('task-123', trackedTask);
    slack.approvalMessageTsByTask.set('task-123', '1710000000.300');
    slack.approvalInFlight.add('task-123');
    updateMessage.mockResolvedValue({});
    vi.spyOn(slack as never, 'isTerminalOrbitEvent').mockReturnValue(true);

    await slack.handleOrbitTaskEvent(
      createEvent({
        event: 'approval.resolved',
        status: 'running',
        payload: {
          action: 'cancel',
          worker_status: 'cancelled',
          worker_id: 'worker-123',
          channel_id: 'C123',
          repository: 'orbit/slack',
        },
      }),
      trackedTask
    );

    expect(updateMessage).toHaveBeenCalledWith({
      channel: 'C123',
      ts: '1710000000.300',
      text: expect.stringContaining('Approval resolved'),
      blocks: expect.any(Array),
    });
    expect(postMessage).not.toHaveBeenCalled();
    expect(untrackTask).toHaveBeenCalledWith('task-123');
    expect(slack.trackedTasks.has('task-123')).toBe(false);
    expect(slack.approvalMessageTsByTask.has('task-123')).toBe(false);
    expect(slack.approvalInFlight.has('task-123')).toBe(false);
    expect(slack.approvalResolved.has('task-123')).toBe(false);
  });

  it('posts a new message when an approval is resolved without a known approval message ts', async () => {
    const { slack, postMessage, updateMessage } = createTestSlackInterface();
    const trackedTask = createTrackedTask();
    postMessage.mockResolvedValue({ ts: '1710000000.302' });

    await slack.handleOrbitTaskEvent(
      createEvent({
        event: 'approval.resolved',
        status: 'running',
        payload: {
          action: 'cancel',
          worker_status: 'cancelled',
          channel_id: 'C123',
        },
      }),
      trackedTask
    );

    expect(updateMessage).not.toHaveBeenCalled();
    expect(postMessage).toHaveBeenCalledWith({
      channel: 'C123',
      text: 'Approval resolved for task task-123: cancel.',
      blocks: expect.any(Array),
      thread_ts: '1710000000.100',
    });
  });

  it('ignores task-created events because they do not produce Slack messages', async () => {
    const { slack, postMessage } = createTestSlackInterface();

    await slack.handleOrbitTaskEvent(
      createEvent({
        topic: 'task',
        event: 'task.created',
        status: 'pending',
      }),
      createTrackedTask()
    );

    expect(postMessage).not.toHaveBeenCalled();
  });

  it('does not clean up tracked task state when updating a resolved approval message fails', async () => {
    const { slack, updateMessage, untrackTask } = createTestSlackInterface();
    const trackedTask = createTrackedTask();
    slack.trackedTasks.set('task-123', trackedTask);
    slack.approvalMessageTsByTask.set('task-123', '1710000000.300');
    slack.approvalInFlight.add('task-123');
    updateMessage.mockRejectedValue(new Error('slack update failed'));

    await expect(
      slack.handleOrbitTaskEvent(
        createEvent({
          event: 'approval.resolved',
          status: 'completed',
          payload: {
            action: 'cancel',
            worker_status: 'failed',
            worker_id: 'worker-123',
            channel_id: 'C123',
            repository: 'orbit/slack',
          },
        }),
        trackedTask
      )
    ).rejects.toThrow('slack update failed');

    expect(untrackTask).not.toHaveBeenCalled();
    expect(slack.trackedTasks.get('task-123')).toEqual(trackedTask);
    expect(slack.approvalMessageTsByTask.get('task-123')).toBe('1710000000.300');
    expect(slack.approvalInFlight.has('task-123')).toBe(false);
    expect(slack.approvalResolved.has('task-123')).toBe(true);
  });

  it('cleans up tracked task state after terminal lane events', async () => {
    const { slack, postMessage, untrackTask } = createTestSlackInterface();
    const trackedTask = createTrackedTask();
    slack.trackedTasks.set('task-123', trackedTask);
    slack.approvalMessageTsByTask.set('task-123', '1710000000.300');
    slack.approvalInFlight.add('task-123');
    slack.approvalResolved.add('task-123');
    postMessage.mockResolvedValue({ ts: '1710000000.400' });

    await slack.handleOrbitTaskEvent(
      createEvent({
        topic: 'lane',
        event: 'lane.green',
        status: 'completed',
        payload: {
          channel_id: 'C123',
          repository: 'orbit/slack',
        },
      }),
      trackedTask
    );

    expect(postMessage).toHaveBeenCalledWith({
      channel: 'C123',
      text: 'Task task-123 (orbit/slack) reported a green lane.',
      blocks: undefined,
      thread_ts: '1710000000.100',
    });
    expect(slack.trackedTasks.has('task-123')).toBe(false);
    expect(slack.approvalMessageTsByTask.has('task-123')).toBe(false);
    expect(slack.approvalInFlight.has('task-123')).toBe(false);
    expect(slack.approvalResolved.has('task-123')).toBe(false);
    expect(untrackTask).toHaveBeenCalledWith('task-123');
  });

  it('falls back to Orbit task snapshots when an event arrives for an unknown task', async () => {
    const { slack, getTask, trackTask } = createTestSlackInterface();
    getTask.mockResolvedValue(
      createTaskSnapshot({
        task_id: 'task-unknown',
        channel_id: 'C999',
        thread_ts: '1710000000.900',
        user_id: 'U999',
        approval_message_ts: '1710000001.000',
      })
    );

    const resolvedTask = await slack.resolveTrackedTaskForEvent(
      createEvent({
        task_id: 'task-unknown',
        payload: {},
      })
    );

    expect(getTask).toHaveBeenCalledWith('task-unknown');
    expect(trackTask).toHaveBeenCalledWith({
      taskId: 'task-unknown',
      channelId: 'C999',
      threadTs: '1710000000.900',
      userId: 'U999',
    });
    expect(resolvedTask).toEqual({
      taskId: 'task-unknown',
      channelId: 'C999',
      threadTs: '1710000000.900',
      userId: 'U999',
    });
    expect(slack.approvalMessageTsByTask.get('task-unknown')).toBe('1710000001.000');
  });

  it('returns no tracked task when an event arrives without a task id', async () => {
    const { slack, getTask } = createTestSlackInterface();

    const resolvedTask = await slack.resolveTrackedTaskForEvent(
      createEvent({
        task_id: undefined,
      })
    );

    expect(resolvedTask).toBeUndefined();
    expect(getTask).not.toHaveBeenCalled();
  });

  it('hydrates a known tracked task from event summary routing fields', async () => {
    const { slack } = createTestSlackInterface();
    slack.trackedTasks.set(
      'task-123',
      createTrackedTask({
        channelId: 'C123',
        threadTs: undefined,
        userId: undefined,
      })
    );

    const resolvedTask = await slack.resolveTrackedTaskForEvent(
      createEvent({
        payload: {
          channel_id: 'C999',
          thread_ts: '1710000000.999',
          user_id: 'U999',
          repository: 'orbit/slack',
        },
      })
    );

    expect(resolvedTask).toEqual({
      taskId: 'task-123',
      channelId: 'C999',
      threadTs: '1710000000.999',
      userId: 'U999',
    });
    expect(slack.trackedTasks.get('task-123')).toEqual(resolvedTask);
  });

  it('creates tracked routing directly from event summaries without calling Orbit task lookup', async () => {
    const { slack, getTask, trackTask } = createTestSlackInterface();

    const resolvedTask = await slack.resolveTrackedTaskForEvent(
      createEvent({
        task_id: 'task-456',
        payload: {
          channel_id: 'C456',
          thread_ts: '1710000000.456',
          user_id: 'U456',
        },
      })
    );

    expect(getTask).not.toHaveBeenCalled();
    expect(trackTask).toHaveBeenCalledWith({
      taskId: 'task-456',
      channelId: 'C456',
      threadTs: '1710000000.456',
      userId: 'U456',
    });
    expect(resolvedTask).toEqual({
      taskId: 'task-456',
      channelId: 'C456',
      threadTs: '1710000000.456',
      userId: 'U456',
    });
  });

  it('derives Slack task routing directly from event summaries before fetching Orbit task snapshots', async () => {
    const { slack, getTask, trackTask } = createTestSlackInterface();

    const resolvedTask = await slack.resolveTrackedTaskForEvent(
      createEvent({
        task_id: 'task-from-event',
        payload: {
          channel_id: 'C777',
          thread_ts: '1710000000.777',
          user_id: 'U777',
          repository: 'orbit/slack',
        },
      })
    );

    expect(getTask).not.toHaveBeenCalled();
    expect(trackTask).toHaveBeenCalledWith({
      taskId: 'task-from-event',
      channelId: 'C777',
      threadTs: '1710000000.777',
      userId: 'U777',
    });
    expect(resolvedTask).toEqual({
      taskId: 'task-from-event',
      channelId: 'C777',
      threadTs: '1710000000.777',
      userId: 'U777',
    });
    expect(slack.trackedTasks.get('task-from-event')).toEqual(resolvedTask);
  });

  it('returns no tracked task when fallback Orbit lookup fails', async () => {
    const warnSpy = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const { slack, getTask } = createTestSlackInterface();
    getTask.mockRejectedValue(new Error('not found'));

    const resolvedTask = await slack.resolveTrackedTaskForEvent(
      createEvent({
        task_id: 'task-missing',
        payload: {},
      })
    );

    expect(resolvedTask).toBeUndefined();
    expect(warnSpy).toHaveBeenCalledWith(
      'Failed to resolve Slack task routing from Orbit event',
      expect.objectContaining({
        taskId: 'task-missing',
        error: 'not found',
      })
    );
  });

  it('returns no tracked task when the Orbit snapshot cannot be converted into Slack routing', async () => {
    const { slack, getTask, trackTask } = createTestSlackInterface();
    getTask.mockResolvedValue(
      createTaskSnapshot({
        task_id: 'task-no-routing',
        channel_id: undefined,
        thread_ts: '1710000000.901',
        user_id: 'U901',
      })
    );

    const resolvedTask = await slack.resolveTrackedTaskForEvent(
      createEvent({
        task_id: 'task-no-routing',
        payload: {},
      })
    );

    expect(resolvedTask).toBeUndefined();
    expect(trackTask).not.toHaveBeenCalled();
    expect(slack.trackedTasks.has('task-no-routing')).toBe(false);
  });

  it('hydrates active Slack tasks from Orbit during startup sync', async () => {
    const { slack, listTasks, trackTask } = createTestSlackInterface();
    listTasks.mockResolvedValue([
      createTaskSnapshot({
        task_id: 'task-1',
        channel_id: 'C101',
        thread_ts: '1710000000.101',
        user_id: 'U101',
        approval_message_ts: '1710000000.201',
        worker_status: 'running',
      }),
      createTaskSnapshot({
        task_id: 'task-2',
        channel_id: 'C202',
        thread_ts: '1710000000.102',
        user_id: 'U202',
        approval_message_ts: '1710000000.202',
        worker_status: 'orphaned',
      }),
    ]);

    await slack.syncTrackedTasksFromOrbit();

    expect(listTasks).toHaveBeenCalledWith({
      source: 'slack',
      status: 'pending,running',
    });
    expect(trackTask).toHaveBeenCalledTimes(2);
    expect(slack.trackedTasks.get('task-1')).toEqual({
      taskId: 'task-1',
      channelId: 'C101',
      threadTs: '1710000000.101',
      userId: 'U101',
    });
    expect(slack.approvalMessageTsByTask.get('task-2')).toBe('1710000000.202');
    expect(slack.approvalResolved.has('task-1')).toBe(true);
    expect(slack.approvalResolved.has('task-2')).toBe(false);
  });

  it('skips sync entries that do not have a Slack channel id', async () => {
    const { slack, listTasks, trackTask } = createTestSlackInterface();
    listTasks.mockResolvedValue([
      createTaskSnapshot({
        task_id: 'task-no-channel',
        channel_id: undefined,
      }),
    ]);

    await slack.syncTrackedTasksFromOrbit();

    expect(trackTask).not.toHaveBeenCalled();
    expect(slack.trackedTasks.size).toBe(0);
  });

  it('logs and continues when startup sync from Orbit fails', async () => {
    const warnSpy = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const { slack, listTasks, trackTask } = createTestSlackInterface();
    listTasks.mockRejectedValue(new Error('orbit down'));

    await slack.syncTrackedTasksFromOrbit();

    expect(trackTask).not.toHaveBeenCalled();
    expect(warnSpy).toHaveBeenCalledWith('Failed to synchronize Slack tasks from Orbit', {
      error: 'orbit down',
    });
  });

  it('hydrates known tracked tasks with newer routing details from later events', async () => {
    const { slack, getTask, trackTask } = createTestSlackInterface();
    slack.trackedTasks.set(
      'task-123',
      createTrackedTask({
        channelId: 'C123',
        threadTs: undefined,
        userId: undefined,
      })
    );

    const resolvedTask = await slack.resolveTrackedTaskForEvent(
      createEvent({
        payload: {
          channel_id: 'C456',
          thread_ts: '1710000000.456',
          user_id: 'U456',
          repository: 'orbit/slack',
        },
      })
    );

    expect(getTask).not.toHaveBeenCalled();
    expect(trackTask).not.toHaveBeenCalled();
    expect(resolvedTask).toEqual({
      taskId: 'task-123',
      channelId: 'C456',
      threadTs: '1710000000.456',
      userId: 'U456',
    });
    expect(slack.trackedTasks.get('task-123')).toEqual(resolvedTask);
  });

  it('preserves an existing thread ts when a newer event omits thread_ts', async () => {
    const { slack } = createTestSlackInterface();
    slack.trackedTasks.set(
      'task-123',
      createTrackedTask({
        channelId: 'C123',
        threadTs: '1710000000.100',
        userId: 'U123',
      })
    );

    const resolvedTask = await slack.resolveTrackedTaskForEvent(
      createEvent({
        payload: {
          channel_id: 'C456',
          user_id: 'U456',
          repository: 'orbit/slack',
        },
      })
    );

    expect(resolvedTask).toEqual({
      taskId: 'task-123',
      channelId: 'C456',
      threadTs: '1710000000.100',
      userId: 'U456',
    });
    expect(slack.trackedTasks.get('task-123')).toEqual(resolvedTask);
  });

  it('returns the original tracked task when an event summary does not change routing fields', () => {
    const { slack } = createTestSlackInterface();
    const task = createTrackedTask();

    expect(
      slack.hydrateTrackedTaskFromEvent(
        task,
        createEvent({
          payload: {
            repository: 'orbit/slack',
          },
        })
      )
    ).toBe(task);
  });

  it('upserts tracked tasks while preserving an existing thread ts when a new task omits it', () => {
    const { slack } = createTestSlackInterface();
    slack.trackedTasks.set('task-123', createTrackedTask());

    slack.upsertTrackedTask(
      createTrackedTask({
        channelId: 'C456',
        threadTs: undefined,
        userId: 'U456',
      })
    );

    expect(slack.trackedTasks.get('task-123')).toEqual({
      taskId: 'task-123',
      channelId: 'C456',
      threadTs: '1710000000.100',
      userId: 'U456',
    });
  });
});
