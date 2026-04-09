import { beforeEach, describe, expect, it, vi } from 'vitest';
import { OrbitApiClient } from '../src/api-client';
import type { OrbitTask, OrbitUpdateTaskContextRequest, SlackBlock, SlackBody } from '../src/types';

interface MockHttpClient {
  get: ReturnType<typeof vi.fn>;
  post: ReturnType<typeof vi.fn>;
}

function installMockHttpClient(client: OrbitApiClient, mockHttpClient: MockHttpClient): void {
  Object.defineProperty(client, 'client', {
    value: mockHttpClient,
    configurable: true,
  });
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

describe('OrbitApiClient', () => {
  let client: OrbitApiClient;
  let mockHttpClient: MockHttpClient;

  beforeEach(() => {
    client = new OrbitApiClient();
    mockHttpClient = {
      get: vi.fn(),
      post: vi.fn(),
    };
    installMockHttpClient(client, mockHttpClient);
  });

  it('builds a hosted events websocket URL with slack-scoped query params', () => {
    const url = client.getEventsWebSocketUrl({
      source: 'slack',
      status: 'running',
      repository: 'orbit/slack',
      limit: 25,
      thread_ts: '',
    });

    expect(url).toBe(
      'ws://localhost:8787/v1/events/ws?source=slack&status=running&repository=orbit%2Fslack&limit=25'
    );
  });

  it('upgrades https hosted API URLs to wss for the events stream', () => {
    const secureClient = new OrbitApiClient();
    Object.defineProperty(secureClient, 'baseUrl', {
      value: 'https://orbit.example.com/api',
      configurable: true,
    });

    expect(secureClient.getEventsWebSocketUrl({ source: 'slack' })).toBe(
      'wss://orbit.example.com/v1/events/ws?source=slack'
    );
  });

  it('submits prompts through the hosted prompt route', async () => {
    const responseBody = {
      ok: true,
      args: ['prompt'],
      duration_ms: 12,
      stdout: 'done',
      stderr: '',
    };
    mockHttpClient.post.mockResolvedValue({ data: responseBody });

    const response = await client.submitPrompt({
      prompt: 'Fix the failing Slack test',
      provider: 'openai',
    });

    expect(mockHttpClient.post).toHaveBeenCalledWith(
      '/v1/prompt',
      {
        prompt: 'Fix the failing Slack test',
        provider: 'openai',
      },
      expect.objectContaining({
        headers: expect.anything(),
      })
    );
    expect(response).toEqual(responseBody);
  });

  it('wraps prompt submission failures with an Orbit API error', async () => {
    mockHttpClient.post.mockRejectedValue(new Error('timeout'));

    await expect(
      client.submitPrompt({
        prompt: 'Fix the failing Slack test',
      })
    ).rejects.toThrow('Orbit API error: timeout');
  });

  it('runs CLI commands through the hosted CLI route', async () => {
    const responseBody = {
      ok: true,
      args: ['status'],
      duration_ms: 6,
      stdout: 'healthy',
      stderr: '',
    };
    mockHttpClient.post.mockResolvedValue({ data: responseBody });

    const response = await client.runCliCommand({
      args: ['status'],
      force_json_output: true,
    });

    expect(mockHttpClient.post).toHaveBeenCalledWith(
      '/v1/cli/run',
      {
        args: ['status'],
        force_json_output: true,
      },
      expect.objectContaining({
        headers: expect.anything(),
      })
    );
    expect(response).toEqual(responseBody);
  });

  it('wraps CLI command failures with an Orbit API error', async () => {
    mockHttpClient.post.mockRejectedValue(new Error('cli timeout'));

    await expect(
      client.runCliCommand({
        args: ['status'],
      })
    ).rejects.toThrow('Orbit API error: cli timeout');
  });

  it('loads hosted status and version with request timing headers', async () => {
    const status = {
      system: {
        status: 'healthy' as const,
        version: '1.2.3',
        uptime: 42,
      },
      tasks: {
        total_tasks: 10,
        active_tasks: 3,
        completed_tasks: 6,
        failed_tasks: 1,
      },
    };
    const version = {
      version: '1.2.3',
      commit: 'abc123',
      build_time: '2026-04-09T10:00:00Z',
    };
    mockHttpClient.get.mockResolvedValueOnce({ data: status }).mockResolvedValueOnce({ data: version });

    await expect(client.getStatus()).resolves.toEqual(status);
    await expect(client.getVersion()).resolves.toEqual(version);

    expect(mockHttpClient.get).toHaveBeenNthCalledWith(
      1,
      '/v1/status',
      expect.objectContaining({
        headers: expect.anything(),
      })
    );
    expect(mockHttpClient.get).toHaveBeenNthCalledWith(
      2,
      '/v1/version',
      expect.objectContaining({
        headers: expect.anything(),
      })
    );
  });

  it('wraps version lookup failures with an Orbit API error', async () => {
    mockHttpClient.get.mockRejectedValue(new Error('version unavailable'));

    await expect(client.getVersion()).rejects.toThrow('Orbit API error: version unavailable');
  });

  it('wraps hosted status, sandbox, and version failures with an Orbit API error', async () => {
    mockHttpClient.get
      .mockRejectedValueOnce(new Error('status down'))
      .mockRejectedValueOnce(new Error('sandbox down'))
      .mockRejectedValueOnce(new Error('version down'));

    await expect(client.getStatus()).rejects.toThrow('Orbit API error: status down');
    await expect(client.getSandboxStatus()).rejects.toThrow('Orbit API error: sandbox down');
    await expect(client.getVersion()).rejects.toThrow('Orbit API error: version down');
  });

  it('creates tasks and resolves approvals through the hosted task routes', async () => {
    const created = {
      task_id: 'task-123',
      status: 'running' as const,
      message: 'created',
      lane_id: 'lane-123',
      worker_id: 'worker-123',
      worker_status: 'running',
    };
    const approved = createTaskSnapshot({
      status: 'cancelled',
      worker_status: 'cancelled',
    });
    mockHttpClient.post.mockResolvedValueOnce({ data: created }).mockResolvedValueOnce({ data: approved });

    await expect(
      client.createTask({
        prompt: 'Investigate flaky test',
        source: 'slack',
        channel_id: 'C123',
      })
    ).resolves.toEqual(created);

    await expect(
      client.resolveTaskApproval({
        taskId: 'task-123',
        approvalKind: 'orphaned_hosted_agent',
        action: 'cancel',
        resolvedBy: 'U123',
        reason: 'User approved cancellation',
      })
    ).resolves.toEqual(approved);

    expect(mockHttpClient.post).toHaveBeenNthCalledWith(1, '/v1/tasks', {
      prompt: 'Investigate flaky test',
      source: 'slack',
      channel_id: 'C123',
    });
    expect(mockHttpClient.post).toHaveBeenNthCalledWith(2, '/v1/tasks/task-123/approval', {
      approval_kind: 'orphaned_hosted_agent',
      action: 'cancel',
      resolved_by: 'U123',
      reason: 'User approved cancellation',
    });
  });

  it('looks up a single task and the effective orphan policy', async () => {
    const task = createTaskSnapshot({
      source: 'slack',
      channel_id: 'C123',
    });
    const policy = {
      preview: {
        repository: 'orbit/slack',
      },
      default_policy: {
        source: 'default',
      },
      effective_policy: {
        source: 'rule',
        repository: 'orbit/slack',
      },
      configured_rules: [
        {
          repository: 'orbit/slack',
          auto_retry_after_secs: 30,
        },
      ],
    };
    mockHttpClient.get.mockResolvedValueOnce({ data: task }).mockResolvedValueOnce({ data: policy });

    await expect(client.getTask('task-123')).resolves.toEqual(task);
    await expect(client.getOrphanPolicy({ repository: 'orbit/slack' })).resolves.toEqual(policy);

    expect(mockHttpClient.get).toHaveBeenNthCalledWith(1, '/v1/tasks/task-123');
    expect(mockHttpClient.get).toHaveBeenNthCalledWith(2, '/v1/policies/orphans', {
      params: {
        repository: 'orbit/slack',
      },
    });
  });

  it('passes list task filters through to the hosted tasks endpoint', async () => {
    const tasks = [createTaskSnapshot()];
    mockHttpClient.get.mockResolvedValue({ data: tasks });

    const response = await client.listTasks({
      status: 'pending,running',
      source: 'slack',
      channel_id: 'C123',
      repository: 'orbit/slack',
    });

    expect(mockHttpClient.get).toHaveBeenCalledWith('/v1/tasks', {
      params: {
        status: 'pending,running',
        source: 'slack',
        channel_id: 'C123',
        repository: 'orbit/slack',
      },
    });
    expect(response).toEqual(tasks);
  });

  it('posts task context updates to the hosted task context route', async () => {
    const request: OrbitUpdateTaskContextRequest = {
      taskId: 'task-123',
      source: 'slack',
      user_id: 'U123',
      channel_id: 'C123',
      thread_ts: '1710000000.100',
      approval_message_ts: '1710000000.200',
    };
    const task = createTaskSnapshot({
      source: 'slack',
      channel_id: 'C123',
      thread_ts: '1710000000.100',
      approval_message_ts: '1710000000.200',
    });
    mockHttpClient.post.mockResolvedValue({ data: task });

    const response = await client.updateTaskContext(request);

    expect(mockHttpClient.post).toHaveBeenCalledWith('/v1/tasks/task-123/context', {
      source: 'slack',
      user_id: 'U123',
      channel_id: 'C123',
      thread_ts: '1710000000.100',
      approval_message_ts: '1710000000.200',
    });
    expect(response).toEqual(task);
  });

  it('sends connector interactions through the canonical hosted connector route', async () => {
    const blocks: SlackBlock[] = [
      {
        type: 'section',
        text: {
          type: 'mrkdwn',
          text: '*Approval required*',
        },
      },
    ];
    const request: SlackBody = {
      user: { id: 'U123' },
      channel: { id: 'C123' },
      message: { ts: '1710000000.100' },
    };
    mockHttpClient.post.mockResolvedValue({ data: { blocks } });

    const response = await client.sendConnectorInteraction('slack', {
      action: 'orphaned_hosted_agent.retry',
      value: 'task-123',
      userId: 'U123',
      context: request,
    });

    expect(mockHttpClient.post).toHaveBeenCalledWith('/v1/connectors/slack/interactions', {
      action: 'orphaned_hosted_agent.retry',
      value: 'task-123',
      user_id: 'U123',
      context: request,
    });
    expect(response).toEqual({ blocks });
  });

  it('sends connector events through the canonical hosted connector event route', async () => {
    mockHttpClient.post.mockResolvedValue({ data: undefined });

    await client.sendConnectorEvent('slack', {
      type: 'reaction_added',
      userId: 'U123',
      data: { reaction: 'eyes' },
    });

    expect(mockHttpClient.post).toHaveBeenCalledWith('/v1/connectors/slack/events', {
      type: 'reaction_added',
      user_id: 'U123',
      data: { reaction: 'eyes' },
    });
  });

  it('returns false when the hosted health check fails', async () => {
    mockHttpClient.get.mockRejectedValue(new Error('down'));

    const healthy = await client.healthCheck();

    expect(healthy).toBe(false);
  });

  it('returns false when the hosted health endpoint responds without HTTP 200', async () => {
    mockHttpClient.get.mockResolvedValue({ status: 503 });

    await expect(client.healthCheck()).resolves.toBe(false);
  });

  it('returns true when the hosted health check succeeds', async () => {
    mockHttpClient.get.mockResolvedValue({ status: 200 });

    await expect(client.healthCheck()).resolves.toBe(true);
  });

  it('returns true when the hosted health check succeeds', async () => {
    mockHttpClient.get.mockResolvedValue({ status: 200 });

    const healthy = await client.healthCheck();

    expect(mockHttpClient.get).toHaveBeenCalledWith('/health');
    expect(healthy).toBe(true);
  });

  it('delegates checkSandboxStatus to the hosted sandbox endpoint', async () => {
    const sandbox = {
      status: 'ready' as const,
      workspaces: 2,
      active_sessions: 1,
    };
    mockHttpClient.get.mockResolvedValue({ data: sandbox });

    const response = await client.checkSandboxStatus();

    expect(mockHttpClient.get).toHaveBeenCalledWith(
      '/v1/sandbox',
      expect.objectContaining({
        headers: expect.anything(),
      })
    );
    expect(response).toEqual(sandbox);
  });

  it('rethrows checkSandboxStatus failures after logging them', async () => {
    const failure = new Error('sandbox unavailable');
    const sandboxSpy = vi
      .spyOn(client, 'getSandboxStatus')
      .mockRejectedValue(failure);

    await expect(client.checkSandboxStatus()).rejects.toBe(failure);
    expect(sandboxSpy).toHaveBeenCalled();
  });

  it('rethrows wrapped sandbox status errors from checkSandboxStatus', async () => {
    mockHttpClient.get.mockRejectedValue(new Error('sandbox unavailable'));

    await expect(client.checkSandboxStatus()).rejects.toThrow(
      'Orbit API error: sandbox unavailable'
    );
  });

  it('rethrows task and connector route failures without wrapping them', async () => {
    const failure = new Error('request failed');
    mockHttpClient.get
      .mockRejectedValueOnce(failure)
      .mockRejectedValueOnce(failure)
      .mockRejectedValueOnce(failure);
    mockHttpClient.post
      .mockRejectedValueOnce(failure)
      .mockRejectedValueOnce(failure)
      .mockRejectedValueOnce(failure)
      .mockRejectedValueOnce(failure)
      .mockRejectedValueOnce(failure);

    await expect(client.getTask('task-123')).rejects.toBe(failure);
    await expect(client.getOrphanPolicy({ source: 'slack' })).rejects.toBe(failure);
    await expect(client.listTasks({ source: 'slack' })).rejects.toBe(failure);
    await expect(
      client.createTask({
        prompt: 'Investigate flaky test',
      })
    ).rejects.toBe(failure);
    await expect(
      client.updateTaskContext({
        taskId: 'task-123',
        channel_id: 'C123',
      })
    ).rejects.toBe(failure);
    await expect(
      client.sendConnectorInteraction('slack', {
        action: 'orphaned_hosted_agent.retry',
        userId: 'U123',
        context: {
          user: { id: 'U123' },
          channel: { id: 'C123' },
          message: { ts: '1710000000.100' },
        },
      })
    ).rejects.toBe(failure);
    await expect(
      client.sendConnectorEvent('slack', {
        type: 'reaction_added',
        userId: 'U123',
        data: { reaction: 'eyes' },
      })
    ).rejects.toBe(failure);
    await expect(
      client.resolveTaskApproval({
        taskId: 'task-123',
        approvalKind: 'orphaned_hosted_agent',
        action: 'retry',
      })
    ).rejects.toBe(failure);
  });

  it('rethrows remaining hosted task and connector route failures without wrapping them', async () => {
    const failure = new Error('request failed');
    mockHttpClient.get.mockRejectedValueOnce(failure);
    mockHttpClient.post
      .mockRejectedValueOnce(failure)
      .mockRejectedValueOnce(failure)
      .mockRejectedValueOnce(failure);

    await expect(
      client.createTask({
        prompt: 'Investigate flaky test',
        source: 'slack',
        channel_id: 'C123',
      })
    ).rejects.toBe(failure);
    await expect(client.listTasks({ source: 'slack' })).rejects.toBe(failure);
    await expect(
      client.updateTaskContext({
        taskId: 'task-123',
        source: 'slack',
        channel_id: 'C123',
      })
    ).rejects.toBe(failure);
    await expect(
      client.sendConnectorEvent('slack', {
        type: 'reaction_added',
        userId: 'U123',
        data: { reaction: 'eyes' },
      })
    ).rejects.toBe(failure);
  });

  it('builds a secure hosted events websocket URL from an https base URL', () => {
    const secureClient = new OrbitApiClient();
    Object.defineProperty(secureClient, 'baseUrl', {
      value: 'https://orbit.example.com/internal/api/',
      configurable: true,
    });

    const url = secureClient.getEventsWebSocketUrl({
      source: 'slack',
      channel_id: 'C123',
      thread_ts: '',
      limit: 10,
      cursor: undefined,
    });

    expect(url).toBe(
      'wss://orbit.example.com/v1/events/ws?source=slack&channel_id=C123&limit=10'
    );
  });
});
