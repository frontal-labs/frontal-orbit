import { describe, expect, it } from 'vitest';
import { SlackInterface } from '../src/slack';
import type {
  OrbitAppliedOrphanPolicy,
  OrbitApprovalAction,
  OrbitEventEnvelope,
  OrbitEventTaskSummary,
  OrbitOrphanPolicyResponse,
  OrbitTrackedTask,
} from '../src/types';

type TestableSlackInterface = SlackInterface & {
  formatOrbitEvent(
    event: OrbitEventEnvelope,
    task: OrbitTrackedTask
  ): {
    text: string;
    blocks?: Array<{
      type: string;
      text?: { text: string };
      elements?: Array<{ action_id?: string; value?: string }>;
    }>;
  } | null;
  parseOrphanPolicyCommand(
    text: string
  ): { repository?: string; source?: string; priority?: string } | null;
  buildOrphanPolicyCommandResponse(policy: OrbitOrphanPolicyResponse): {
    response_type: 'ephemeral';
    text: string;
    blocks: Array<{ type: string; text?: { text: string } }>;
  };
  describeOrphanPolicy(policy?: OrbitAppliedOrphanPolicy): string | undefined;
  formatOrphanPolicyRuleLines(policy: OrbitOrphanPolicyResponse): string[];
  readEventTaskSummary(event: OrbitEventEnvelope): OrbitEventTaskSummary;
  formatLaneStartedEvent(
    taskLabel: string,
    event: OrbitEventEnvelope,
    summary: OrbitEventTaskSummary
  ): {
    text: string;
  };
  formatLaneFailedEvent(
    taskLabel: string,
    event: OrbitEventEnvelope,
    summary: OrbitEventTaskSummary
  ): {
    text: string;
  };
  formatTaskCreatedMessage(task: { task_id: string; plan_kind?: string; worker_status?: string; worker_id?: string }, prompt: string): string;
  formatTaskRoutedEvent(
    taskLabel: string,
    event: OrbitEventEnvelope,
    summary: OrbitEventTaskSummary
  ): {
    text: string;
  };
  formatLaneBlockedEvent(taskLabel: string, event: OrbitEventEnvelope): {
    text: string;
  };
  formatApprovalResolvedEvent(
    task: OrbitTrackedTask,
    taskLabel: string,
    event: OrbitEventEnvelope,
    summary: OrbitEventTaskSummary
  ): {
    text: string;
    blocks?: Array<{
      type: string;
      text?: { text: string };
      elements?: Array<{ action_id?: string; value?: string }>;
    }>;
  };
  buildApprovalProcessingBlocks(
    taskId: string,
    action: OrbitApprovalAction
  ): Array<{
    type: string;
    text?: { text: string };
  }>;
  buildApprovalResolvedBlocks(
    taskId: string,
    action: string,
    workerStatus?: string,
    workerId?: string
  ): Array<{
    type: string;
    text?: { text: string };
  }>;
  buildApprovalErrorBlocks(
    taskId: string,
    error: Error
  ): Array<{
    type: string;
    text?: { text: string };
    elements?: Array<{ action_id?: string; value?: string }>;
  }>;
  formatApprovalRequestedEvent(
    event: OrbitEventEnvelope,
    task: OrbitTrackedTask
  ): {
    text: string;
    blocks?: Array<{
      type: string;
      text?: { text: string };
      elements?: Array<{ action_id?: string; value?: string }>;
    }>;
  };
};

function createSlackInterface(): TestableSlackInterface {
  return Object.create(SlackInterface.prototype) as TestableSlackInterface;
}

describe('SlackInterface formatting helpers', () => {
  it('parses orphan policy slash-command filters from shorthand tokens', () => {
    const slack = createSlackInterface();

    const query = slack.parseOrphanPolicyCommand(
      'policy orphans repo=orbit/slack source=slack priority=high'
    );

    expect(query).toEqual({
      repository: 'orbit/slack',
      source: 'slack',
      priority: 'high',
    });
  });

  it('ignores malformed and unknown orphan policy command tokens', () => {
    const slack = createSlackInterface();

    const query = slack.parseOrphanPolicyCommand(
      'policy orphans repo=orbit/slack source= priority=high invalid unknown=value'
    );

    expect(query).toEqual({
      repository: 'orbit/slack',
      priority: 'high',
    });
  });

  it('returns null for empty or non-policy orphan commands', () => {
    const slack = createSlackInterface();

    expect(slack.parseOrphanPolicyCommand('')).toBeNull();
    expect(slack.parseOrphanPolicyCommand('policy')).toBeNull();
    expect(slack.parseOrphanPolicyCommand('hello world')).toBeNull();
  });

  it('ignores malformed and unknown orphan policy tokens while preserving valid filters', () => {
    const slack = createSlackInterface();

    const query = slack.parseOrphanPolicyCommand(
      'policy orphans foo=bar repo=orbit/slack broken source= priority=high'
    );

    expect(query).toEqual({
      repository: 'orbit/slack',
      priority: 'high',
    });
  });

  it('renders orphan policy previews with effective policy and rule details', () => {
    const slack = createSlackInterface();
    const response = slack.buildOrphanPolicyCommandResponse({
      preview: {
        repository: 'orbit/slack',
        source: 'slack',
        priority: 'high',
      },
      default_policy: {
        source: 'default',
        approval_delay_secs: 60,
      },
      effective_policy: {
        source: 'rule',
        match_repository: 'orbit/slack',
        match_source: 'slack',
        approval_delay_secs: 30,
        auto_retry_after_secs: 90,
      },
      configured_rules: [
        {
          repository: 'orbit/slack',
          source: 'slack',
          approval_delay_secs: 30,
          auto_retry_after_secs: 90,
        },
      ],
    });

    expect(response.response_type).toBe('ephemeral');
    expect(response.text).toContain('repo=orbit/slack');
    expect(response.blocks[1]?.text?.text).toContain('1. repo=orbit/slack, source=slack');
  });

  it('renders orphan policy previews without selectors or scoped rules', () => {
    const slack = createSlackInterface();
    const response = slack.buildOrphanPolicyCommandResponse({
      default_policy: {
        source: 'default',
        approval_delay_secs: 60,
      },
      effective_policy: {
        source: 'default',
        approval_delay_secs: 60,
      },
      configured_rules: [],
    });

    expect(response.text).toContain('Orphan policy: Policy: default; approval 60s.');
    expect(response.blocks[1]?.text?.text).toContain('No scoped rules configured.');
  });

  it('renders orphan policy previews when only non-repository selectors are present', () => {
    const slack = createSlackInterface();
    const response = slack.buildOrphanPolicyCommandResponse({
      preview: {
        source: 'slack',
        priority: 'high',
      },
      default_policy: {
        source: 'default',
        approval_delay_secs: 60,
      },
      effective_policy: {
        source: 'rule',
        match_source: 'slack',
        match_priority: 'high',
        approval_delay_secs: 30,
      },
      configured_rules: [],
    });

    expect(response.text).toContain('preview for source=slack, priority=high');
    expect(response.blocks[0]?.text?.text).toContain('Selectors: source=slack, priority=high');
  });

  it('renders policy unavailable fallbacks when effective and default policies are missing', () => {
    const slack = createSlackInterface();
    const response = slack.buildOrphanPolicyCommandResponse({
      preview: {
        repository: 'orbit/slack',
      },
      effective_policy: undefined,
      default_policy: undefined,
      configured_rules: [],
    });

    expect(response.text).toContain('preview for repo=orbit/slack: Policy unavailable.');
    expect(response.blocks[0]?.text?.text).toContain('Effective: Policy unavailable.');
    expect(response.blocks[0]?.text?.text).toContain('Default: Policy unavailable.');
    expect(response.blocks[1]?.text?.text).toContain('No scoped rules configured.');
  });

  it('formats orphan policy text with selectors and optional retry/cancel timing', () => {
    const slack = createSlackInterface();

    expect(
      slack.describeOrphanPolicy({
        source: 'rule',
        match_repository: 'orbit/slack',
        match_source: 'slack',
        match_priority: 'high',
        approval_delay_secs: 30,
        auto_retry_after_secs: 90,
        auto_cancel_after_secs: 300,
      })
    ).toBe(
      'Policy: rule (repo=orbit/slack, source=slack, priority=high); approval 30s, auto-retry 90s, auto-cancel 300s.'
    );
    expect(slack.describeOrphanPolicy()).toBeUndefined();
  });

  it('formats orphan policy rule lines with inherit-defaults fallback and truncates after five rules', () => {
    const slack = createSlackInterface();

    const lines = slack.formatOrphanPolicyRuleLines({
      default_policy: {
        source: 'default',
        approval_delay_secs: 60,
      },
      effective_policy: {
        source: 'default',
        approval_delay_secs: 60,
      },
      configured_rules: [
        {},
        { repository: 'repo-1', approval_delay_secs: 10 },
        { repository: 'repo-2', auto_retry_after_secs: 20 },
        { repository: 'repo-3', auto_cancel_after_secs: 30 },
        { source: 'slack', priority: 'high', approval_delay_secs: 40 },
        { repository: 'repo-6', approval_delay_secs: 50 },
      ],
    });

    expect(lines).toEqual([
      '1. match any -> inherit defaults',
      '2. repo=repo-1 -> approval 10s',
      '3. repo=repo-2 -> retry 20s',
      '4. repo=repo-3 -> cancel 30s',
      '5. source=slack, priority=high -> approval 40s',
    ]);
  });

  it('returns an empty event task summary when the event has no payload', () => {
    const slack = createSlackInterface();

    const summary = slack.readEventTaskSummary({
      event_id: 'evt-no-payload',
      topic: 'lane',
      event: 'lane.failed',
      status: 'failed',
      emittedAt: '2026-04-09T10:10:00Z',
      task_id: 'task-123',
    });

    expect(summary).toEqual({});
  });

  it('normalizes unsupported task_status values while preserving summary fields', () => {
    const slack = createSlackInterface();

    const summary = slack.readEventTaskSummary({
      event_id: 'evt-invalid-status',
      topic: 'lane',
      event: 'lane.failed',
      status: 'failed',
      emittedAt: '2026-04-09T10:10:01Z',
      task_id: 'task-123',
      payload: {
        task_status: 'queued',
        channel_id: 'C123',
        repository: 'orbit/slack',
        worker_status: 'failed',
        worker_id: 'worker-123',
        error: 'provider crashed',
      },
    });

    expect(summary).toEqual({
      task_status: undefined,
      source: undefined,
      user_id: undefined,
      channel_id: 'C123',
      thread_ts: undefined,
      approval_message_ts: undefined,
      repository: 'orbit/slack',
      branch: undefined,
      priority: undefined,
      plan_id: undefined,
      plan_kind: undefined,
      work_item_id: undefined,
      worker_id: 'worker-123',
      worker_status: 'failed',
      result: undefined,
      error: 'provider crashed',
    });
  });

  it('preserves supported task_status values in event summaries', () => {
    const slack = createSlackInterface();

    const summary = slack.readEventTaskSummary({
      event_id: 'evt-valid-status',
      topic: 'lane',
      event: 'lane.started',
      status: 'running',
      emittedAt: '2026-04-09T10:10:01Z',
      task_id: 'task-123',
      payload: {
        task_status: 'running',
        channel_id: 'C123',
      },
    });

    expect(summary.task_status).toBe('running');
    expect(summary.channel_id).toBe('C123');
  });

  it('formats lane-started events from payload fallbacks when summary fields are absent', () => {
    const slack = createSlackInterface();

    const message = slack.formatLaneStartedEvent(
      'Task task-123',
      {
        event_id: 'evt-lane-started',
        topic: 'lane',
        event: 'lane.started',
        status: 'running',
        emittedAt: '2026-04-09T10:10:02Z',
        task_id: 'task-123',
        payload: {
          role: 'reviewer',
          worker_status: 'booting',
          worker_id: 'worker-123',
        },
      },
      {}
    );

    expect(message).toEqual({
      text: 'Task task-123 reviewer started with worker booting (worker-123).',
    });
  });

  it('formats task-created messages with worker status when no worker id is present', () => {
    const slack = createSlackInterface();

    expect(
      slack.formatTaskCreatedMessage(
        {
          task_id: 'task-123',
          worker_status: 'ready_for_prompt',
        },
        'Investigate flaky test'
      )
    ).toContain('Worker: ready_for_prompt');
  });

  it('formats routed events from payload plan kind without lane counts', () => {
    const slack = createSlackInterface();

    const message = slack.formatTaskRoutedEvent(
      'Task task-123',
      {
        event_id: 'evt-routed-fallback',
        topic: 'task',
        event: 'task.routed',
        status: 'running',
        emittedAt: '2026-04-09T10:10:02Z',
        task_id: 'task-123',
        payload: {
          plan_kind: 'reviewer',
        },
      },
      {}
    );

    expect(message).toEqual({
      text: 'Task task-123 routed to reviewer.',
    });
  });

  it('formats routed events with a positive lane count suffix', () => {
    const slack = createSlackInterface();

    const message = slack.formatTaskRoutedEvent(
      'Task task-123',
      {
        event_id: 'evt-routed-lanes',
        topic: 'task',
        event: 'task.routed',
        status: 'running',
        emittedAt: '2026-04-09T10:10:02Z',
        task_id: 'task-123',
        payload: {
          lane_count: 3,
        },
      },
      {
        plan_kind: 'implementer',
      }
    );

    expect(message).toEqual({
      text: 'Task task-123 routed to implementer (3 lanes).',
    });
  });

  it('falls back to the assigned plan kind when routed events have no summary or payload plan kind', () => {
    const slack = createSlackInterface();

    const message = slack.formatTaskRoutedEvent(
      'Task task-123',
      {
        event_id: 'evt-routed-default',
        topic: 'task',
        event: 'task.routed',
        status: 'running',
        emittedAt: '2026-04-09T10:10:02Z',
        task_id: 'task-123',
      },
      {}
    );

    expect(message).toEqual({
      text: 'Task task-123 routed to assigned.',
    });
  });

  it('prefers summary error details over payload error text when formatting lane failures', () => {
    const slack = createSlackInterface();

    const message = slack.formatLaneFailedEvent(
      'Task task-123',
      {
        event_id: 'evt-lane-failed',
        topic: 'lane',
        event: 'lane.failed',
        status: 'failed',
        emittedAt: '2026-04-09T10:10:02Z',
        task_id: 'task-123',
        payload: {
          error: 'payload error',
        },
      },
      {
        error: 'summary error',
      }
    );

    expect(message).toEqual({
      text: 'Task task-123 failed: summary error',
    });
  });

  it('falls back to a default lane failure message when no error details exist', () => {
    const slack = createSlackInterface();

    const message = slack.formatLaneFailedEvent(
      'Task task-123',
      {
        event_id: 'evt-lane-failed-default',
        topic: 'lane',
        event: 'lane.failed',
        status: 'failed',
        emittedAt: '2026-04-09T10:10:03Z',
        task_id: 'task-123',
      },
      {}
    );

    expect(message).toEqual({
      text: 'Task task-123 failed: lane execution failed',
    });
  });

  it('formats blocked lane events from detail fallback without orphan policy text', () => {
    const slack = createSlackInterface();

    const message = slack.formatLaneBlockedEvent('Task task-123', {
      event_id: 'evt-lane-blocked-detail',
      topic: 'lane',
      event: 'lane.blocked',
      status: 'waiting',
      emittedAt: '2026-04-09T10:10:04Z',
      task_id: 'task-123',
      payload: {
        detail: 'executor stalled',
      },
    });

    expect(message).toEqual({
      text: 'Task task-123 is blocked: executor stalled',
    });
  });

  it('falls back to the default blocked-lane reason when neither reason nor detail is present', () => {
    const slack = createSlackInterface();

    const message = slack.formatLaneBlockedEvent('Task task-123', {
      event_id: 'evt-lane-blocked-default',
      topic: 'lane',
      event: 'lane.blocked',
      status: 'waiting',
      emittedAt: '2026-04-09T10:10:04Z',
      task_id: 'task-123',
    });

    expect(message).toEqual({
      text: 'Task task-123 is blocked: waiting for input',
    });
  });

  it('formats approval resolution from payload fallbacks when summary data is absent', () => {
    const slack = createSlackInterface();

    const message = slack.formatApprovalResolvedEvent(
      {
        taskId: 'task-123',
        channelId: 'C123',
      },
      'Task task-123',
      {
        event_id: 'evt-approval-resolved-fallback',
        topic: 'approval',
        event: 'approval.resolved',
        status: 'completed',
        emittedAt: '2026-04-09T10:10:05Z',
        task_id: 'task-123',
        payload: {
          worker_status: 'ready_for_prompt',
          worker_id: 'worker-123',
        },
      },
      {}
    );

    expect(message.text).toBe('Approval resolved for task task-123: updated.');
    expect(message.blocks?.[0]?.text?.text).toContain('Worker: ready_for_prompt (worker-123)');
  });

  it('formats github follow-up resolution separately from orphan approvals', () => {
    const slack = createSlackInterface();

    const message = slack.formatApprovalResolvedEvent(
      {
        taskId: 'task-123',
        channelId: 'C123',
      },
      'Task task-123',
      {
        event_id: 'evt-github-followup-resolved',
        topic: 'approval',
        event: 'approval.resolved',
        status: 'completed',
        emittedAt: '2026-04-09T10:10:05Z',
        task_id: 'task-123',
        payload: {
          approval_kind: 'github_review_followup',
          action: 'cleared',
        },
      },
      {}
    );

    expect(message.text).toBe('GitHub follow-up cleared for task task-123.');
    expect(message.blocks?.[0]?.text?.text).toContain('Action: `cleared`');
  });

  it('builds approval processing blocks for orphan approvals', () => {
    const slack = createSlackInterface();

    const blocks = slack.buildApprovalProcessingBlocks('task-123', 'retry');

    expect(blocks[0]?.text?.text).toContain('Processing approval for task task-123');
    expect(blocks[0]?.text?.text).toContain('retry');
  });

  it('builds approval resolved blocks with both worker status and worker id', () => {
    const slack = createSlackInterface();

    const blocks = slack.buildApprovalResolvedBlocks(
      'task-123',
      'retry',
      'ready_for_prompt',
      'worker-123'
    );

    expect(blocks[0]?.text?.text).toContain('Action: `retry`');
    expect(blocks[0]?.text?.text).toContain('Worker: ready_for_prompt (worker-123)');
  });

  it('builds approval resolved blocks with only worker status', () => {
    const slack = createSlackInterface();

    const blocks = slack.buildApprovalResolvedBlocks('task-123', 'cancel', 'cancelled');

    expect(blocks[0]?.text?.text).toContain('Action: `cancel`');
    expect(blocks[0]?.text?.text).toContain('Worker: cancelled');
  });

  it('builds approval resolved blocks without worker details', () => {
    const slack = createSlackInterface();

    const blocks = slack.buildApprovalResolvedBlocks('task-123', 'already_resolved');

    expect(blocks[0]?.text?.text).toContain('Action: `already_resolved`');
    expect(blocks[0]?.text?.text).not.toContain('Worker:');
  });

  it('builds approval error blocks with retry and cancel actions', () => {
    const slack = createSlackInterface();

    const blocks = slack.buildApprovalErrorBlocks('task-123', new Error('provider failed'));
    const actions = blocks.find((block) => block.type === 'actions');

    expect(blocks[0]?.text?.text).toContain('provider failed');
    expect(actions?.elements?.map((element) => element.action_id)).toEqual([
      'orphaned_hosted_agent.retry',
      'orphaned_hosted_agent.cancel',
    ]);
  });

  it('formats orphaned approval requests with retry and cancel actions', () => {
    const slack = createSlackInterface();
    const event: OrbitEventEnvelope = {
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
        repository: 'orbit/slack',
        orphan_policy: {
          source: 'rule',
          match_repository: 'orbit/slack',
          approval_delay_secs: 60,
          auto_cancel_after_secs: 300,
        },
      },
    };
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const message = slack.formatApprovalRequestedEvent(event, task);
    const actions = message.blocks?.find((block) => block.type === 'actions');

    expect(message.text).toContain('needs approval');
    expect(actions?.elements?.map((element) => element.action_id)).toEqual([
      'orphaned_hosted_agent.retry',
      'orphaned_hosted_agent.cancel',
    ]);
    expect(actions?.elements?.map((element) => element.value)).toEqual(['task-123', 'task-123']);
  });

  it('formats non-orphan approval requests as a simple waiting message', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const message = slack.formatApprovalRequestedEvent(
      {
        event_id: 'evt-124',
        topic: 'approval',
        event: 'approval.requested',
        status: 'waiting',
        emittedAt: '2026-04-09T10:00:01Z',
        task_id: 'task-123',
        payload: {
          approval_kind: 'human_review',
          channel_id: 'C123',
          repository: 'orbit/slack',
        },
      },
      task
    );

    expect(message).toEqual({
      text: 'Task task-123 (orbit/slack) is waiting for approval.',
    });
  });

  it('formats github review follow-up requests with follow-up buttons', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const message = slack.formatApprovalRequestedEvent(
      {
        event_id: 'evt-github-followup-requested',
        topic: 'approval',
        event: 'approval.requested',
        status: 'waiting',
        emittedAt: '2026-04-09T10:00:01Z',
        task_id: 'task-123',
        payload: {
          approval_kind: 'github_review_followup',
          reason: 'GitHub review requested changes.',
          repository: 'orbit/slack',
        },
      },
      task
    );

    expect(message.text).toBe(
      'Task task-123 (orbit/slack) needs follow-up: GitHub review requested changes.'
    );
    const actions = message.blocks?.find((block) => block.type === 'actions');
    expect(actions?.elements?.map((element) => element.action_id)).toEqual([
      'github_review_followup.ack',
      'github_review_followup.retry',
    ]);
    expect(actions?.elements?.map((element) => element.value)).toEqual([
      'task-123',
      'task-123',
    ]);
  });

  it('formats blocked lane events with orphan policy detail in the message text', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const message = slack.formatOrbitEvent(
      {
        event_id: 'evt-125',
        topic: 'lane',
        event: 'lane.blocked',
        status: 'waiting',
        emittedAt: '2026-04-09T10:00:02Z',
        task_id: 'task-123',
        payload: {
          channel_id: 'C123',
          repository: 'orbit/slack',
          detail: 'executor heartbeat expired',
          orphan_policy: {
            source: 'rule',
            match_repository: 'orbit/slack',
            approval_delay_secs: 60,
            auto_cancel_after_secs: 300,
          },
        },
      },
      task
    );

    expect(message?.text).toContain(
      'Task task-123 (orbit/slack) is blocked: executor heartbeat expired'
    );
    expect(message?.text).toContain(
      'Policy: rule (repo=orbit/slack); approval 60s, auto-cancel 300s.'
    );
  });

  it('formats cancelled task events with orphan policy detail when present', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const message = slack.formatOrbitEvent(
      {
        event_id: 'evt-131',
        topic: 'task',
        event: 'task.cancelled',
        status: 'cancelled',
        emittedAt: '2026-04-09T10:00:08Z',
        task_id: 'task-123',
        payload: {
          channel_id: 'C123',
          repository: 'orbit/slack',
          orphan_policy: {
            source: 'rule',
            match_repository: 'orbit/slack',
            approval_delay_secs: 60,
            auto_cancel_after_secs: 300,
          },
        },
      },
      task
    );

    expect(message).toEqual({
      text: 'Task task-123 (orbit/slack) was cancelled. Policy: rule (repo=orbit/slack); approval 60s, auto-cancel 300s.',
    });
  });

  it('formats cancelled task events without orphan policy detail when absent', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const message = slack.formatOrbitEvent(
      {
        event_id: 'evt-131b',
        topic: 'task',
        event: 'task.cancelled',
        status: 'cancelled',
        emittedAt: '2026-04-09T10:00:08Z',
        task_id: 'task-123',
        payload: {
          channel_id: 'C123',
        },
      },
      task
    );

    expect(message).toEqual({
      text: 'Task task-123 was cancelled.',
    });
  });

  it('returns null for task.created and non-github connector notifications', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    expect(
      slack.formatOrbitEvent(
        {
          event_id: 'evt-132',
          topic: 'task',
          event: 'task.created',
          status: 'pending',
          emittedAt: '2026-04-09T10:00:09Z',
          task_id: 'task-123',
        },
        task
      )
    ).toBeNull();

    expect(
      slack.formatOrbitEvent(
        {
          event_id: 'evt-133',
          topic: 'connector',
          event: 'connector.event.received',
          status: 'running',
          emittedAt: '2026-04-09T10:00:10Z',
          task_id: 'task-123',
          payload: {
            connector: 'slack',
            type: 'reaction_added',
          },
        },
        task
      )
    ).toBeNull();
  });

  it('formats task-bound github connector events for PR updates, merges, reviews, and comments', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const prUpdate = slack.formatOrbitEvent(
      {
        event_id: 'evt-133',
        topic: 'connector',
        event: 'connector.event.received',
        status: 'completed',
        emittedAt: '2026-04-09T10:00:10Z',
        task_id: 'task-123',
        payload: {
          connector: 'github',
          type: 'pull_request.synchronize',
          data: {
            pr_number: 42,
            sender_login: 'octocat',
            html_url: 'https://github.com/acme/payments/pull/42',
          },
        },
      },
      task
    );

    expect(prUpdate).toEqual({
      text: 'Task task-123 received a GitHub PR update (#42) from octocat. https://github.com/acme/payments/pull/42',
    });

    const merged = slack.formatOrbitEvent(
      {
        event_id: 'evt-133a',
        topic: 'connector',
        event: 'connector.event.received',
        status: 'completed',
        emittedAt: '2026-04-09T10:00:10Z',
        task_id: 'task-123',
        payload: {
          connector: 'github',
          type: 'pull_request.closed',
          data: {
            pr_number: 42,
            sender_login: 'octocat',
            pr_merged: true,
            html_url: 'https://github.com/acme/payments/pull/42',
          },
        },
      },
      task
    );

    expect(merged).toEqual({
      text: 'Task task-123 linked GitHub PR #42 was merged by octocat. https://github.com/acme/payments/pull/42',
    });

    const review = slack.formatOrbitEvent(
      {
        event_id: 'evt-133b',
        topic: 'connector',
        event: 'connector.event.received',
        status: 'completed',
        emittedAt: '2026-04-09T10:00:10Z',
        task_id: 'task-123',
        payload: {
          connector: 'github',
          type: 'pull_request_review.submitted',
          data: {
            pr_number: 42,
            sender_login: 'reviewer',
            review_state: 'APPROVED',
            review_body: 'Ship it',
          },
        },
      },
      task
    );

    expect(review).toEqual({
      text: 'Task task-123 received a GitHub review on PR #42 from reviewer (approved): Ship it',
    });

    const comment = slack.formatOrbitEvent(
      {
        event_id: 'evt-134',
        topic: 'connector',
        event: 'connector.event.received',
        status: 'completed',
        emittedAt: '2026-04-09T10:00:11Z',
        task_id: 'task-123',
        payload: {
          connector: 'github',
          type: 'issue_comment.created',
          data: {
            pr_number: 42,
            sender_login: 'reviewer',
            comment_body: 'Looks good to me',
          },
        },
      },
      task
    );

    expect(comment).toEqual({
      text: 'Task task-123 received a GitHub comment on PR #42 from reviewer: Looks good to me',
    });
  });

  it('falls back to a generic update message for unrecognized event types', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const message = slack.formatOrbitEvent(
      {
        event_id: 'evt-134',
        topic: 'task',
        event: 'task.reconciled',
        status: 'running',
        emittedAt: '2026-04-09T10:00:11Z',
        task_id: 'task-123',
        payload: {
          repository: 'orbit/slack',
        },
      },
      task
    );

    expect(message).toEqual({
      text: 'Task task-123 (orbit/slack) updated: task.reconciled',
    });
  });

  it('formats rule lines with inherited defaults when no timings are set', () => {
    const slack = createSlackInterface();
    const response = slack.buildOrphanPolicyCommandResponse({
      default_policy: {
        source: 'default',
        approval_delay_secs: 60,
      },
      effective_policy: {
        source: 'default',
        approval_delay_secs: 60,
      },
      configured_rules: [
        {
          source: 'slack',
        },
      ],
    });

    expect(response.blocks[1]?.text?.text).toContain('1. source=slack -> inherit defaults');
  });

  it('falls back to the default lane failure message when no error details are present', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const message = slack.formatOrbitEvent(
      {
        event_id: 'evt-126',
        topic: 'lane',
        event: 'lane.failed',
        status: 'failed',
        emittedAt: '2026-04-09T10:00:03Z',
        task_id: 'task-123',
        payload: {
          channel_id: 'C123',
        },
      },
      task
    );

    expect(message).toEqual({
      text: 'Task task-123 failed: lane execution failed',
    });
  });

  it('formats lane started events with explicit role and worker details', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const message = slack.formatOrbitEvent(
      {
        event_id: 'evt-129',
        topic: 'lane',
        event: 'lane.started',
        status: 'running',
        emittedAt: '2026-04-09T10:00:06Z',
        task_id: 'task-123',
        payload: {
          channel_id: 'C123',
          repository: 'orbit/slack',
          role: 'reviewer',
          worker_status: 'ready_for_prompt',
          worker_id: 'worker-123',
        },
      },
      task
    );

    expect(message).toEqual({
      text: 'Task task-123 (orbit/slack) reviewer started with worker ready_for_prompt (worker-123).',
    });
  });

  it('falls back to the generic lane role and event status for lane started messages', () => {
    const slack = createSlackInterface();
    const task: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    };

    const message = slack.formatOrbitEvent(
      {
        event_id: 'evt-130',
        topic: 'lane',
        event: 'lane.started',
        status: 'booting',
        emittedAt: '2026-04-09T10:00:07Z',
        task_id: 'task-123',
        payload: {
          channel_id: 'C123',
        },
      },
      task
    );

    expect(message).toEqual({
      text: 'Task task-123 lane started with worker booting.',
    });
  });

  it('reads empty task summaries when the event has no payload and ignores invalid task statuses', () => {
    const slack = createSlackInterface();
    const noPayloadMessage = slack.formatOrbitEvent(
      {
        event_id: 'evt-127',
        topic: 'memory',
        event: 'memory.captured',
        status: 'completed',
        emittedAt: '2026-04-09T10:00:04Z',
        task_id: 'task-123',
      },
      {
        taskId: 'task-123',
        channelId: 'C123',
      }
    );

    const invalidStatusMessage = slack.formatOrbitEvent(
      {
        event_id: 'evt-128',
        topic: 'task',
        event: 'task.routed',
        status: 'running',
        emittedAt: '2026-04-09T10:00:05Z',
        task_id: 'task-123',
        payload: {
          task_status: 'unknown_status',
          plan_kind: 'reviewer',
        },
      },
      {
        taskId: 'task-123',
        channelId: 'C123',
      }
    );

    expect(noPayloadMessage).toEqual({
      text: 'Memory captured for task task-123.',
    });
    expect(invalidStatusMessage).toEqual({
      text: 'Task task-123 routed to reviewer.',
    });
  });
});
