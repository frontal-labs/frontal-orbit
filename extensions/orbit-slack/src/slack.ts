import { App } from "@slack/bolt";
import { OrbitApiClient } from "./api-client";
import { config } from "./config";
import { logger } from "./log";
import { OrbitEventsClient } from "./orbit-events";
import type {
  OrbitAppliedOrphanPolicy,
  OrbitApprovalAction,
  OrbitCreateTaskResponse,
  OrbitEventEnvelope,
  OrbitEventPayload,
  OrbitEventTaskSummary,
  OrbitOrphanPolicyQuery,
  OrbitOrphanPolicyResponse,
  OrbitTask,
  OrbitTrackedTask,
  SlackBlock,
  SlackBody,
} from "./types";

// Type definitions for Slack Bolt handlers
interface SlackMessage {
  text: string;
  user: string;
  channel: string;
  ts: string;
  thread_ts?: string;
  bot_id?: string;
  subtype?: string;
}

interface SlackAction {
  action_id: string;
  value?: string;
}

interface SlackMessagePayload {
  text: string;
  blocks?: SlackBlock[];
}

// Pure Slack interface - WebSocket connection to Slack, talks to Orbit AI
export class SlackInterface {
  private readonly app: App;
  private readonly orbitApi: OrbitApiClient;
  private readonly orbitEvents: OrbitEventsClient;
  private readonly trackedTasks = new Map<string, OrbitTrackedTask>();
  private readonly approvalMessageTsByTask = new Map<string, string>();
  private readonly approvalInFlight = new Set<string>();
  private readonly approvalResolved = new Set<string>();

  constructor() {
    // Initialize Slack WebSocket connection
    this.app = new App({
      token: config.slack.botToken,
      appToken: config.slack.appToken,
      signingSecret: config.slack.signingSecret,
      socketMode: true, // WebSocket connection
      logLevel: undefined, // Minimal logging
    });

    this.orbitApi = new OrbitApiClient();
    this.orbitEvents = new OrbitEventsClient(
      (query) => this.orbitApi.getEventsWebSocketUrl(query),
      () => this.orbitApi.getEventsWebSocketHeaders()
    );
    this.orbitEvents.onTrackedTaskEvent(async (event, task) => {
      await this.handleOrbitTaskEvent(event, task);
    });
    this.setupEventHandlers();
  }

  private setupEventHandlers(): void {
    // Handle all Slack events through WebSocket
    this.app.message(async ({ message }) => {
      // Forward messages to Orbit AI
      const slackMessage = message as SlackMessage;
      if (
        !slackMessage.text ||
        !slackMessage.user ||
        slackMessage.bot_id ||
        slackMessage.subtype
      ) {
        return;
      }

      const threadTs = slackMessage.thread_ts || slackMessage.ts;
      const task = await this.orbitApi.createTask({
        prompt: slackMessage.text,
        user_id: slackMessage.user,
        channel_id: slackMessage.channel,
        thread_ts: threadTs,
        source: "slack",
      });

      await this.registerTask(
        task,
        {
          taskId: task.task_id,
          channelId: slackMessage.channel,
          threadTs,
          userId: slackMessage.user,
        },
        slackMessage.text
      );
    });

    // Handle slash commands
    this.app.command("/ai", async ({ command, ack }) => {
      const policyQuery = this.parseOrphanPolicyCommand(command.text);
      if (policyQuery) {
        try {
          const policy = await this.orbitApi.getOrphanPolicy(policyQuery);
          await ack(this.buildOrphanPolicyCommandResponse(policy));
        } catch (error) {
          await ack({
            response_type: "ephemeral",
            text: `Failed to load orphan policy: ${(error as Error).message}`,
          });
        }
        return;
      }

      await ack();

      const task = await this.orbitApi.createTask({
        prompt: command.text,
        user_id: command.user_id,
        channel_id: command.channel_id,
        source: "slack",
      });

      await this.registerTask(
        task,
        {
          taskId: task.task_id,
          channelId: command.channel_id,
          userId: command.user_id,
        },
        command.text
      );
    });

    // Handle interactions (buttons, modals, etc.)
    this.app.action({ action_id: /^.*$/ }, async ({ action, ack, body }) => {
      await ack();
      await this.handleSlackAction(
        action as SlackAction,
        body as unknown as SlackBody
      );
    });

    // Handle events (user joins, reactions, etc.)
    this.app.event(/.*/, async ({ event }) => {
      // Forward events to Orbit AI for processing
      const slackEvent = event as { type: string; user?: string };
      await this.orbitApi.sendConnectorEvent("slack", {
        type: slackEvent.type,
        userId: slackEvent.user || "",
        data: event,
      });
    });
  }

  // Start WebSocket connection to Slack
  async connect(): Promise<void> {
    try {
      await this.syncTrackedTasksFromOrbit();
      await this.app.start();
      await this.orbitEvents.connect();
      logger.info("Slack WebSocket interface connected");
    } catch (error) {
      logger.error("Failed to connect to Slack", error as Error);
      throw error;
    }
  }

  // Disconnect WebSocket
  async disconnect(): Promise<void> {
    try {
      await this.orbitEvents.disconnect();
      await this.app.stop();
      logger.info("Slack WebSocket interface disconnected");
    } catch (error) {
      logger.error("Failed to disconnect from Slack", error as Error);
      throw error;
    }
  }

  // Health check
  async healthCheck(): Promise<{ slack: boolean; orbit: boolean }> {
    try {
      const slackConnected =
        (
          this.app as unknown as { isListening?: () => boolean }
        ).isListening?.() || false;
      const orbitConnected = await this.orbitApi.healthCheck();

      return {
        slack: slackConnected,
        orbit: orbitConnected,
      };
    } catch (error) {
      logger.error("Health check failed", error as Error);
      return {
        slack: false,
        orbit: false,
      };
    }
  }

  private async registerTask(
    task: OrbitCreateTaskResponse,
    trackedTask: OrbitTrackedTask,
    prompt: string
  ): Promise<void> {
    const message = this.formatTaskCreatedMessage(task, prompt);
    const response = await this.app.client.chat.postMessage({
      channel: trackedTask.channelId,
      text: message,
      thread_ts: trackedTask.threadTs,
    });

    const nextTask = {
      ...trackedTask,
      threadTs: trackedTask.threadTs || response.ts,
    };
    this.upsertTrackedTask(nextTask);
    try {
      await this.orbitApi.updateTaskContext({
        taskId: nextTask.taskId,
        source: "slack",
        user_id: nextTask.userId,
        channel_id: nextTask.channelId,
        thread_ts: nextTask.threadTs,
      });
    } catch (error) {
      logger.warn("Failed to persist Slack task thread anchor to Orbit", {
        taskId: nextTask.taskId,
        error: (error as Error).message,
      });
    }
    this.orbitEvents.trackTask(nextTask);
  }

  private async handleSlackAction(
    action: SlackAction,
    body: SlackBody
  ): Promise<void> {
    if (this.isOrphanApprovalAction(action)) {
      await this.handleOrphanApprovalAction(action, body);
      return;
    }
    if (this.isGithubFollowupApprovalAction(action)) {
      await this.handleGithubFollowupApprovalAction(action, body);
      return;
    }

    const response = await this.orbitApi.sendConnectorInteraction("slack", {
      action: action.action_id,
      value: action.value,
      userId: body.user.id,
      context: body,
    });

    await this.app.client.chat.update({
      channel: body.channel.id,
      ts: body.message.ts,
      blocks: response.blocks,
    });
  }

  private isOrphanApprovalAction(action: SlackAction): boolean {
    return (
      Boolean(action.value) &&
      (action.action_id === "orphaned_hosted_agent.retry" ||
        action.action_id === "orphaned_hosted_agent.cancel")
    );
  }

  private isGithubFollowupApprovalAction(action: SlackAction): boolean {
    return (
      Boolean(action.value) &&
      (action.action_id === "github_review_followup.ack" ||
        action.action_id === "github_review_followup.retry")
    );
  }

  private async handleOrphanApprovalAction(
    action: SlackAction,
    body: SlackBody
  ): Promise<void> {
    const taskId = action.value;
    if (!taskId) {
      return;
    }

    const actionName: OrbitApprovalAction = action.action_id.endsWith(".retry")
      ? "retry"
      : "cancel";
    const approvalTs =
      this.approvalMessageTsByTask.get(taskId) || body.message.ts;

    if (this.approvalResolved.has(taskId)) {
      await this.updateApprovalMessage(
        body.channel.id,
        approvalTs,
        `Approval for task ${taskId} was already resolved.`,
        this.buildApprovalResolvedBlocks(taskId, "already_resolved")
      );
      return;
    }

    if (this.approvalInFlight.has(taskId)) {
      await this.updateApprovalMessage(
        body.channel.id,
        approvalTs,
        `Approval for task ${taskId} is already being processed.`,
        this.buildApprovalProcessingBlocks(taskId, actionName)
      );
      return;
    }

    this.approvalInFlight.add(taskId);
    this.approvalMessageTsByTask.set(taskId, approvalTs);

    await this.updateApprovalMessage(
      body.channel.id,
      approvalTs,
      `Processing approval for task ${taskId}: ${actionName}.`,
      this.buildApprovalProcessingBlocks(taskId, actionName)
    );

    try {
      const task = await this.orbitApi.resolveTaskApproval({
        taskId,
        approvalKind: "orphaned_hosted_agent",
        action: actionName,
        resolvedBy: body.user.id,
        reason: "resolved from Slack approval action",
      });

      this.approvalInFlight.delete(taskId);
      this.approvalResolved.add(taskId);

      await this.updateApprovalMessage(
        body.channel.id,
        approvalTs,
        `Approval resolved for task ${task.task_id}: ${actionName}.`,
        this.buildApprovalResolvedBlocks(
          task.task_id,
          actionName,
          task.worker_status,
          task.worker_id
        )
      );
    } catch (error) {
      this.approvalInFlight.delete(taskId);
      await this.updateApprovalMessage(
        body.channel.id,
        approvalTs,
        `Approval failed for task ${taskId}.`,
        this.buildApprovalErrorBlocks(taskId, error as Error)
      );
      throw error;
    }
  }

  private async handleGithubFollowupApprovalAction(
    action: SlackAction,
    body: SlackBody
  ): Promise<void> {
    const taskId = action.value;
    if (!taskId) {
      return;
    }
    const actionName: OrbitApprovalAction = action.action_id.endsWith(".retry")
      ? "retry"
      : "ack";
    const approvalTs =
      this.approvalMessageTsByTask.get(taskId) || body.message.ts;

    if (this.approvalResolved.has(taskId)) {
      await this.updateApprovalMessage(
        body.channel.id,
        approvalTs,
        `Follow-up for task ${taskId} was already resolved.`,
        this.buildApprovalResolvedBlocks(taskId, "already_resolved")
      );
      return;
    }

    if (this.approvalInFlight.has(taskId)) {
      await this.updateApprovalMessage(
        body.channel.id,
        approvalTs,
        `Follow-up for task ${taskId} is already being processed.`,
        this.buildApprovalProcessingBlocks(taskId, actionName)
      );
      return;
    }

    this.approvalInFlight.add(taskId);
    try {
      const result = await this.orbitApi.resolveTaskApproval({
        taskId,
        approvalKind: "github_review_followup",
        action: actionName,
        resolvedBy: body.user.name || body.user.id,
      });
      this.approvalResolved.add(taskId);
      this.approvalInFlight.delete(taskId);
      await this.updateApprovalMessage(
        body.channel.id,
        approvalTs,
        `Follow-up resolved for task ${taskId}.`,
        this.buildApprovalResolvedBlocks(taskId, actionName)
      );
      this.trackTask(result);
    } catch (error) {
      this.approvalInFlight.delete(taskId);
      await this.updateApprovalMessage(
        body.channel.id,
        approvalTs,
        `Follow-up for task ${taskId} failed: ${(error as Error).message}`,
        this.buildApprovalErrorBlocks(taskId, error as Error)
      );
    }
  }

  private async updateApprovalMessage(
    channel: string,
    ts: string,
    text: string,
    blocks: SlackBlock[]
  ): Promise<void> {
    await this.app.client.chat.update({
      channel,
      ts,
      text,
      blocks,
    });
  }

  private async handleOrbitTaskEvent(
    event: OrbitEventEnvelope,
    task?: OrbitTrackedTask
  ): Promise<void> {
    const resolvedTask = await this.resolveTrackedTaskForEvent(event, task);
    if (!resolvedTask) {
      return;
    }
    const nextTask = resolvedTask;

    const message = this.formatOrbitEvent(event, nextTask);
    if (!message) {
      return;
    }

    await this.postExternalStatusUpdates(event, message.text);

    if (
      event.event === "approval.requested" &&
      this.approvalResolved.has(nextTask.taskId)
    ) {
      return;
    }

    if (event.event === "approval.resolved") {
      this.approvalInFlight.delete(nextTask.taskId);
      this.approvalResolved.add(nextTask.taskId);
      const approvalTs = this.approvalMessageTsByTask.get(nextTask.taskId);
      if (approvalTs) {
        await this.app.client.chat.update({
          channel: nextTask.channelId,
          text: message.text,
          blocks: message.blocks,
          ts: approvalTs,
        });
        if (this.isTerminalOrbitEvent(event)) {
          await this.cleanupTaskState(nextTask.taskId);
        }
        return;
      }
    }

    const response = await this.app.client.chat.postMessage({
      channel: nextTask.channelId,
      text: message.text,
      blocks: message.blocks,
      thread_ts: nextTask.threadTs,
    });

    if (event.event === "approval.requested") {
      this.approvalMessageTsByTask.set(nextTask.taskId, response.ts as string);
      try {
        await this.orbitApi.updateTaskContext({
          taskId: nextTask.taskId,
          source: "slack",
          user_id: nextTask.userId,
          channel_id: nextTask.channelId,
          thread_ts: nextTask.threadTs,
          approval_message_ts: response.ts as string,
        });
      } catch (error) {
        logger.warn(
          "Failed to persist Slack approval message linkage to Orbit",
          {
            taskId: nextTask.taskId,
            error: (error as Error).message,
          }
        );
      }
    }

    if (this.isTerminalOrbitEvent(event)) {
      await this.cleanupTaskState(nextTask.taskId);
    }
  }

  private async postExternalStatusUpdates(
    event: OrbitEventEnvelope,
    fallbackText: string
  ): Promise<void> {
    const payload = event.payload;
    if (!payload) return;

    const hasLinear =
      payload.linear_issue_id ||
      payload.linear_issue_identifier ||
      payload.linear_issue_url;
    const hasGraphite =
      payload.graphite_stack_id ||
      payload.graphite_head_branch ||
      payload.graphite_base_branch;
    if (!hasLinear && !hasGraphite) return;

    const message =
      this.buildExternalStatusMessage(event) ?? fallbackText ?? undefined;
    if (!message) return;

    if (hasLinear) {
      try {
        await this.orbitApi.postLinearStatus({
          issueId: payload.linear_issue_id,
          identifier: payload.linear_issue_identifier,
          url: payload.linear_issue_url,
          state: payload.linear_issue_state,
          taskId: event.task_id,
          message,
        });
      } catch (error) {
        logger.warn("Failed to post Linear status", {
          error: (error as Error).message,
        });
      }
    }

    if (hasGraphite) {
      try {
        await this.orbitApi.postGraphiteStatus({
          stackId: payload.graphite_stack_id,
          headBranch: payload.graphite_head_branch,
          baseBranch: payload.graphite_base_branch,
          taskId: event.task_id,
          message,
        });
      } catch (error) {
        logger.warn("Failed to post Graphite status", {
          error: (error as Error).message,
        });
      }
    }
  }

  private buildExternalStatusMessage(
    event: OrbitEventEnvelope
  ): string | undefined {
    const summary = this.readEventTaskSummary(event);
    const syntheticTask: OrbitTrackedTask = {
      taskId: event.task_id || summary?.work_item_id || "task",
      channelId: summary?.channel_id || "external",
      threadTs: summary?.thread_ts || undefined,
      userId: summary?.user_id || "",
    };
    const formatted = this.formatOrbitEvent(event, syntheticTask);
    return formatted?.text;
  }

  private async resolveTrackedTaskForEvent(
    event: OrbitEventEnvelope,
    task?: OrbitTrackedTask
  ): Promise<OrbitTrackedTask | undefined> {
    const taskId = event.task_id;
    if (!taskId) {
      return undefined;
    }

    const knownTask = task || this.trackedTasks.get(taskId);
    if (knownTask) {
      const hydratedTask = this.hydrateTrackedTaskFromEvent(knownTask, event);
      this.upsertTrackedTask(hydratedTask);
      return hydratedTask;
    }

    const eventTask = this.readTrackedTaskFromEventSummary(taskId, event);
    if (eventTask) {
      this.upsertTrackedTask(eventTask);
      this.orbitEvents.trackTask(eventTask);
      return eventTask;
    }

    try {
      const snapshot = await this.orbitApi.getTask(taskId);
      const snapshotTask = this.toTrackedTask(snapshot);
      if (!snapshotTask) {
        return undefined;
      }
      this.syncDerivedApprovalState(snapshot);
      if (snapshot.approval_message_ts) {
        this.approvalMessageTsByTask.set(
          snapshotTask.taskId,
          snapshot.approval_message_ts
        );
      }
      this.upsertTrackedTask(snapshotTask);
      this.orbitEvents.trackTask(snapshotTask);
      return snapshotTask;
    } catch (error) {
      logger.warn("Failed to resolve Slack task routing from Orbit event", {
        taskId,
        error: (error as Error).message,
      });
      return undefined;
    }
  }

  private formatTaskCreatedMessage(
    task: OrbitCreateTaskResponse,
    prompt: string
  ): string {
    const parts = [`Task created: ${task.task_id}`, `Prompt: ${prompt}`];

    if (task.plan_kind) {
      parts.push(`Lane: ${task.plan_kind}`);
    }
    if (task.worker_status) {
      parts.push(
        `Worker: ${task.worker_status}${task.worker_id ? ` (${task.worker_id})` : ""}`
      );
    }

    return parts.join("\n");
  }

  private formatOrbitEvent(
    event: OrbitEventEnvelope,
    task: OrbitTrackedTask
  ): SlackMessagePayload | null {
    const summary = this.readEventTaskSummary(event);
    const taskLabel = this.describeTask(task.taskId, summary);

    switch (event.event) {
      case "task.created":
        return null;
      case "task.routed":
        return this.formatTaskRoutedEvent(taskLabel, event, summary);
      case "task.cancelled":
        return this.formatTaskCancelledEvent(taskLabel, event);
      case "lane.started":
        return this.formatLaneStartedEvent(taskLabel, event, summary);
      case "lane.failed":
        return this.formatLaneFailedEvent(taskLabel, event, summary);
      case "lane.blocked":
        return this.formatLaneBlockedEvent(taskLabel, event);
      case "lane.green": {
        const resultText = payload.result || task.result || "";
        const truncated =
          resultText.length > 2800
            ? resultText.substring(0, 2800) + "\n\n... (truncated)"
            : resultText;
        const text = truncated
          ? `${taskLabel} completed:\n\n${truncated}`
          : `${taskLabel} reported a green lane.`;
        return { text };
      }
      case "approval.requested":
        return this.formatApprovalRequestedEvent(event, task);
      case "approval.resolved":
        return this.formatApprovalResolvedEvent(
          task,
          taskLabel,
          event,
          summary
        );
      case "memory.captured":
        return { text: `Memory captured for ${taskLabel.toLowerCase()}.` };
      case "connector.event.received":
        return this.formatConnectorEvent(taskLabel, event);
      default:
        return { text: `${taskLabel} updated: ${event.event}` };
    }
  }

  private formatConnectorEvent(
    taskLabel: string,
    event: OrbitEventEnvelope
  ): SlackMessagePayload | null {
    const payload = event.payload;
    if (!payload || payload.connector !== "github") {
      return null;
    }

    const connectorType = payload.type;
    const data = this.readConnectorEventData(payload.data);
    const actor = this.readConnectorString(data, "user_id", "sender_login");
    const prNumber =
      this.readConnectorNumber(data, "pr_number") ??
      this.readConnectorNumber(data, "number");
    const htmlUrl = this.readConnectorString(data, "html_url");
    const commentBody = this.readConnectorString(data, "comment_body");
    const reviewBody = this.readConnectorString(data, "review_body");
    const reviewState = this.readConnectorString(data, "review_state");
    const prMerged = data?.pr_merged === true;

    switch (connectorType) {
      case "pull_request.synchronize":
        return {
          text: `${taskLabel} received a GitHub PR update${prNumber ? ` (#${prNumber})` : ""}${actor ? ` from ${actor}` : ""}.${htmlUrl ? ` ${htmlUrl}` : ""}`,
        };
      case "pull_request.closed":
        return {
          text: `${taskLabel} linked GitHub PR${prNumber ? ` #${prNumber}` : ""} was ${prMerged ? "merged" : "closed"}${actor ? ` by ${actor}` : ""}.${htmlUrl ? ` ${htmlUrl}` : ""}`,
        };
      case "pull_request.reopened":
        return {
          text: `${taskLabel} linked GitHub PR${prNumber ? ` #${prNumber}` : ""} was reopened${actor ? ` by ${actor}` : ""}.${htmlUrl ? ` ${htmlUrl}` : ""}`,
        };
      case "pull_request_review.submitted":
        return {
          text: `${taskLabel} received a GitHub review${prNumber ? ` on PR #${prNumber}` : ""}${actor ? ` from ${actor}` : ""}${reviewState ? ` (${reviewState.toLowerCase()})` : ""}: ${reviewBody || "new review feedback"}`,
        };
      case "issue_comment.created":
        return {
          text: `${taskLabel} received a GitHub comment${prNumber ? ` on PR #${prNumber}` : ""}${actor ? ` from ${actor}` : ""}: ${commentBody || "new review feedback"}`,
        };
      default:
        return null;
    }
  }

  private formatTaskRoutedEvent(
    taskLabel: string,
    event: OrbitEventEnvelope,
    summary: OrbitEventTaskSummary
  ): SlackMessagePayload {
    const planKind =
      summary.plan_kind || event.payload?.plan_kind || "assigned";
    const laneCount = event.payload?.lane_count;
    return {
      text: `${taskLabel} routed to ${planKind}${laneCount ? ` (${laneCount} lanes)` : ""}.`,
    };
  }

  private formatTaskCancelledEvent(
    taskLabel: string,
    event: OrbitEventEnvelope
  ): SlackMessagePayload {
    const policyText = this.describeOrphanPolicy(
      this.readOrphanPolicy(event.payload)
    );
    return {
      text: `${taskLabel} was cancelled.${policyText ? ` ${policyText}` : ""}`,
    };
  }

  private formatLaneStartedEvent(
    taskLabel: string,
    event: OrbitEventEnvelope,
    summary: OrbitEventTaskSummary
  ): SlackMessagePayload {
    const role = event.payload?.role || "lane";
    const workerStatus =
      summary.worker_status || event.payload?.worker_status || event.status;
    const workerId = summary.worker_id || event.payload?.worker_id;
    return {
      text: `${taskLabel} ${role} started with worker ${workerStatus}${workerId ? ` (${workerId})` : ""}.`,
    };
  }

  private formatLaneFailedEvent(
    taskLabel: string,
    event: OrbitEventEnvelope,
    summary: OrbitEventTaskSummary
  ): SlackMessagePayload {
    const error =
      summary.error || event.payload?.error || "lane execution failed";
    return {
      text: `${taskLabel} failed: ${error}`,
    };
  }

  private formatLaneBlockedEvent(
    taskLabel: string,
    event: OrbitEventEnvelope
  ): SlackMessagePayload {
    const reason =
      event.payload?.reason || event.payload?.detail || "waiting for input";
    const policyText = this.describeOrphanPolicy(
      this.readOrphanPolicy(event.payload)
    );
    return {
      text: `${taskLabel} is blocked: ${reason}${policyText ? ` ${policyText}` : ""}`,
    };
  }

  private formatApprovalResolvedEvent(
    task: OrbitTrackedTask,
    taskLabel: string,
    event: OrbitEventEnvelope,
    summary: OrbitEventTaskSummary
  ): SlackMessagePayload {
    const action = event.payload?.action || "updated";
    const workerStatus = summary.worker_status || event.payload?.worker_status;
    const workerId = summary.worker_id || event.payload?.worker_id;
    if (event.payload?.approval_kind === "github_review_followup") {
      return {
        text: `GitHub follow-up cleared for ${taskLabel.toLowerCase()}.`,
        blocks: this.buildApprovalResolvedBlocks(task.taskId, action),
      };
    }
    return {
      text: `Approval resolved for ${taskLabel.toLowerCase()}: ${action}.`,
      blocks: this.buildApprovalResolvedBlocks(
        task.taskId,
        action,
        workerStatus,
        workerId
      ),
    };
  }

  private formatApprovalRequestedEvent(
    event: OrbitEventEnvelope,
    task: OrbitTrackedTask
  ): SlackMessagePayload {
    const summary = this.readEventTaskSummary(event);
    const payload = event.payload;
    const taskLabel = this.describeTask(task.taskId, summary);
    const approvalKind = payload?.approval_kind;
    const reason = payload?.reason || "Task requires approval.";
    const policyText = this.describeOrphanPolicy(
      this.readOrphanPolicy(payload)
    );

    if (approvalKind === "orphaned_hosted_agent") {
      return {
        text: `${taskLabel} needs approval: ${reason}`,
        blocks: [
          {
            type: "section",
            text: {
              type: "mrkdwn",
              text: `*${taskLabel} needs approval*\n${reason}${policyText ? `\n${policyText}` : ""}`,
            },
          },
          {
            type: "actions",
            elements: [
              {
                type: "button",
                text: {
                  type: "plain_text",
                  text: "Retry Lane",
                  emoji: true,
                },
                style: "primary",
                action_id: "orphaned_hosted_agent.retry",
                value: task.taskId,
              },
              {
                type: "button",
                text: {
                  type: "plain_text",
                  text: "Cancel Task",
                  emoji: true,
                },
                style: "danger",
                action_id: "orphaned_hosted_agent.cancel",
                value: task.taskId,
              },
            ],
          },
        ],
      };
    }

    if (approvalKind === "github_review_followup") {
      return {
        text: `${taskLabel} needs follow-up: ${reason}`,
        blocks: [
          {
            type: "section",
            text: {
              type: "mrkdwn",
              text: `*${taskLabel} needs follow-up*\n${reason}`,
            },
          },
          {
            type: "actions",
            elements: [
              {
                type: "button",
                text: {
                  type: "plain_text",
                  text: "Mark done",
                  emoji: true,
                },
                style: "primary",
                action_id: "github_review_followup.ack",
                value: task.taskId,
              },
              {
                type: "button",
                text: {
                  type: "plain_text",
                  text: "Retry lane",
                  emoji: true,
                },
                action_id: "github_review_followup.retry",
                value: task.taskId,
              },
            ],
          },
        ],
      };
    }

    return {
      text: `${taskLabel} is waiting for approval.`,
    };
  }

  private readEventTaskSummary(
    event: OrbitEventEnvelope
  ): OrbitEventTaskSummary {
    const payload = event.payload;
    if (!payload) {
      return {};
    }

    const taskStatus = payload.task_status;
    return {
      task_status:
        taskStatus === "pending" ||
        taskStatus === "running" ||
        taskStatus === "completed" ||
        taskStatus === "failed" ||
        taskStatus === "cancelled"
          ? taskStatus
          : undefined,
      source: payload.source,
      user_id: payload.user_id,
      channel_id: payload.channel_id,
      thread_ts: payload.thread_ts,
      approval_message_ts: payload.approval_message_ts,
      repository: payload.repository,
      branch: payload.branch,
      priority: payload.priority,
      plan_id: payload.plan_id,
      plan_kind: payload.plan_kind,
      work_item_id: payload.work_item_id,
      worker_id: payload.worker_id,
      worker_status: payload.worker_status,
      result: payload.result,
      error: payload.error,
    };
  }

  private readConnectorEventData(
    data: OrbitEventPayload["data"]
  ): Record<string, unknown> | undefined {
    return data && typeof data === "object" && !Array.isArray(data)
      ? (data as Record<string, unknown>)
      : undefined;
  }

  private readConnectorString(
    data: Record<string, unknown> | undefined,
    ...keys: string[]
  ): string | undefined {
    for (const key of keys) {
      const value = data?.[key];
      if (typeof value === "string" && value.trim()) {
        return value;
      }
    }
    return undefined;
  }

  private readConnectorNumber(
    data: Record<string, unknown> | undefined,
    ...keys: string[]
  ): number | undefined {
    for (const key of keys) {
      const value = data?.[key];
      if (typeof value === "number") {
        return value;
      }
    }
    return undefined;
  }

  private hydrateTrackedTaskFromEvent(
    task: OrbitTrackedTask,
    event: OrbitEventEnvelope
  ): OrbitTrackedTask {
    const summary = this.readEventTaskSummary(event);
    const nextChannelId = summary.channel_id || task.channelId;
    const nextThreadTs = summary.thread_ts || task.threadTs;
    const nextUserId = summary.user_id || task.userId;

    if (
      nextChannelId === task.channelId &&
      nextThreadTs === task.threadTs &&
      nextUserId === task.userId
    ) {
      return task;
    }

    return {
      ...task,
      channelId: nextChannelId,
      threadTs: nextThreadTs,
      userId: nextUserId,
    };
  }

  private readTrackedTaskFromEventSummary(
    taskId: string,
    event: OrbitEventEnvelope
  ): OrbitTrackedTask | undefined {
    const summary = this.readEventTaskSummary(event);
    if (!summary.channel_id) {
      return undefined;
    }

    return {
      taskId,
      channelId: summary.channel_id,
      threadTs: summary.thread_ts,
      userId: summary.user_id,
    };
  }

  private describeTask(taskId: string, summary: OrbitEventTaskSummary): string {
    if (summary.repository) {
      return `Task ${taskId} (${summary.repository})`;
    }
    return `Task ${taskId}`;
  }

  private parseOrphanPolicyCommand(
    text: string
  ): OrbitOrphanPolicyQuery | null {
    const trimmed = text.trim();
    if (!trimmed) {
      return null;
    }

    const parts = trimmed.split(/\s+/);
    if (parts.length < 2 || parts[0] !== "policy" || parts[1] !== "orphans") {
      return null;
    }

    const query: OrbitOrphanPolicyQuery = {};
    for (const token of parts.slice(2)) {
      const [rawKey, ...rest] = token.split("=");
      const rawValue = rest.join("=").trim();
      if (!rawKey || !rawValue) {
        continue;
      }

      const key = rawKey.trim().toLowerCase();
      if (key === "repository" || key === "repo") {
        query.repository = rawValue;
      } else if (key === "source") {
        query.source = rawValue;
      } else if (key === "priority") {
        query.priority = rawValue;
      }
    }

    return query;
  }

  private buildOrphanPolicyCommandResponse(policy: OrbitOrphanPolicyResponse): {
    response_type: "ephemeral";
    text: string;
    blocks: SlackBlock[];
  } {
    const preview = policy.preview
      ? [
          policy.preview.repository
            ? `repo=${policy.preview.repository}`
            : undefined,
          policy.preview.source ? `source=${policy.preview.source}` : undefined,
          policy.preview.priority
            ? `priority=${policy.preview.priority}`
            : undefined,
        ]
          .filter((value): value is string => Boolean(value))
          .join(", ")
      : undefined;

    const ruleLines = this.formatOrphanPolicyRuleLines(policy);

    const effectivePolicyText =
      this.describeOrphanPolicy(policy.effective_policy) ||
      "Policy unavailable.";
    const defaultPolicyText =
      this.describeOrphanPolicy(policy.default_policy) || "Policy unavailable.";

    return {
      response_type: "ephemeral",
      text: `Orphan policy${preview ? ` preview for ${preview}` : ""}: ${effectivePolicyText}`,
      blocks: [
        {
          type: "section",
          text: {
            type: "mrkdwn",
            text: `*Orphan policy${preview ? " preview" : ""}*\n${preview ? `Selectors: ${preview}\n` : ""}Effective: ${effectivePolicyText}\nDefault: ${defaultPolicyText}`,
          },
        },
        {
          type: "section",
          text: {
            type: "mrkdwn",
            text: `*Configured rules*\n${ruleLines.join("\n")}`,
          },
        },
      ],
    };
  }

  private readOrphanPolicy(
    payload?: OrbitEventPayload
  ): OrbitAppliedOrphanPolicy | undefined {
    return payload?.orphan_policy;
  }

  private describeOrphanPolicy(
    policy?: OrbitAppliedOrphanPolicy
  ): string | undefined {
    if (!policy) {
      return undefined;
    }

    const selectors = [
      policy.match_repository ? `repo=${policy.match_repository}` : undefined,
      policy.match_source ? `source=${policy.match_source}` : undefined,
      policy.match_priority ? `priority=${policy.match_priority}` : undefined,
    ].filter((value): value is string => Boolean(value));
    const policyScope =
      selectors.length > 0
        ? `${policy.source} (${selectors.join(", ")})`
        : policy.source;
    const steps = [
      `approval ${policy.approval_delay_secs}s`,
      policy.auto_retry_after_secs !== undefined
        ? `auto-retry ${policy.auto_retry_after_secs}s`
        : undefined,
      policy.auto_cancel_after_secs !== undefined
        ? `auto-cancel ${policy.auto_cancel_after_secs}s`
        : undefined,
    ].filter((value): value is string => Boolean(value));

    return `Policy: ${policyScope}; ${steps.join(", ")}.`;
  }

  private formatOrphanPolicyRuleLines(
    policy: OrbitOrphanPolicyResponse
  ): string[] {
    if (policy.configured_rules.length === 0) {
      return ["No scoped rules configured."];
    }

    return policy.configured_rules
      .slice(0, 5)
      .map((rule, index) => this.formatOrphanPolicyRuleLine(rule, index));
  }

  private formatOrphanPolicyRuleLine(
    rule: OrbitOrphanPolicyResponse["configured_rules"][number],
    index: number
  ): string {
    const selectors = [
      rule.repository ? `repo=${rule.repository}` : undefined,
      rule.source ? `source=${rule.source}` : undefined,
      rule.priority ? `priority=${rule.priority}` : undefined,
    ]
      .filter((value): value is string => Boolean(value))
      .join(", ");
    const timing = [
      rule.approval_delay_secs !== undefined
        ? `approval ${rule.approval_delay_secs}s`
        : undefined,
      rule.auto_retry_after_secs !== undefined
        ? `retry ${rule.auto_retry_after_secs}s`
        : undefined,
      rule.auto_cancel_after_secs !== undefined
        ? `cancel ${rule.auto_cancel_after_secs}s`
        : undefined,
    ]
      .filter((value): value is string => Boolean(value))
      .join(", ");

    return `${index + 1}. ${selectors || "match any"} -> ${timing || "inherit defaults"}`;
  }

  private buildApprovalProcessingBlocks(
    taskId: string,
    action: OrbitApprovalAction
  ): SlackBlock[] {
    return [
      {
        type: "section",
        text: {
          type: "mrkdwn",
          text: `*Processing approval for task ${taskId}*\nSelected action: \`${action}\``,
        },
      },
    ];
  }

  private buildApprovalResolvedBlocks(
    taskId: string,
    action: string,
    workerStatus?: string,
    workerId?: string
  ): SlackBlock[] {
    const detail =
      workerStatus && workerId
        ? `Worker: ${workerStatus} (${workerId})`
        : workerStatus
          ? `Worker: ${workerStatus}`
          : undefined;

    return [
      {
        type: "section",
        text: {
          type: "mrkdwn",
          text: `*Approval resolved for task ${taskId}*\nAction: \`${action}\`${detail ? `\n${detail}` : ""}`,
        },
      },
    ];
  }

  private buildApprovalErrorBlocks(taskId: string, error: Error): SlackBlock[] {
    return [
      {
        type: "section",
        text: {
          type: "mrkdwn",
          text: `*Approval failed for task ${taskId}*\n${error.message}`,
        },
      },
      {
        type: "actions",
        elements: [
          {
            type: "button",
            text: {
              type: "plain_text",
              text: "Retry Lane",
              emoji: true,
            },
            style: "primary",
            action_id: "orphaned_hosted_agent.retry",
            value: taskId,
          },
          {
            type: "button",
            text: {
              type: "plain_text",
              text: "Cancel Task",
              emoji: true,
            },
            style: "danger",
            action_id: "orphaned_hosted_agent.cancel",
            value: taskId,
          },
        ],
      },
    ];
  }

  private async syncTrackedTasksFromOrbit(): Promise<void> {
    try {
      const activeTasks = await this.orbitApi.listTasks({
        source: "slack",
        status: "pending,running",
      });

      let merged = 0;
      for (const task of activeTasks) {
        const trackedTask = this.toTrackedTask(task);
        if (!trackedTask) {
          continue;
        }

        if (!this.trackedTasks.has(trackedTask.taskId)) {
          merged += 1;
        }
        this.upsertTrackedTask(trackedTask);
        if (task.approval_message_ts) {
          this.approvalMessageTsByTask.set(
            trackedTask.taskId,
            task.approval_message_ts
          );
        }
        this.syncDerivedApprovalState(task);
        this.orbitEvents.trackTask(trackedTask);
      }

      logger.info("Synchronized Slack tasks from Orbit", {
        discoveredTasks: activeTasks.length,
        mergedTasks: merged,
        trackedTasks: this.trackedTasks.size,
      });
    } catch (error) {
      logger.warn("Failed to synchronize Slack tasks from Orbit", {
        error: (error as Error).message,
      });
    }
  }

  private toTrackedTask(task: OrbitTask): OrbitTrackedTask | null {
    if (!task.channel_id) {
      return null;
    }

    return {
      taskId: task.task_id,
      channelId: task.channel_id,
      threadTs: task.thread_ts,
      userId: task.user_id,
    };
  }

  private isTerminalOrbitEvent(event: OrbitEventEnvelope): boolean {
    return (
      event.event === "task.cancelled" ||
      event.event === "lane.green" ||
      event.event === "lane.failed"
    );
  }

  private upsertTrackedTask(task: OrbitTrackedTask): void {
    const existing = this.trackedTasks.get(task.taskId);
    const mergedTask = this.sanitizeTrackedTask({
      ...existing,
      ...task,
      threadTs: task.threadTs || existing?.threadTs,
    });
    this.trackedTasks.set(task.taskId, mergedTask);
  }

  private sanitizeTrackedTask(task: OrbitTrackedTask): OrbitTrackedTask {
    return {
      taskId: task.taskId,
      channelId: task.channelId,
      threadTs: task.threadTs,
      userId: task.userId,
    };
  }

  private syncDerivedApprovalState(task: OrbitTask): void {
    const hasApprovalMessage = Boolean(task.approval_message_ts);
    const awaitingApproval = task.worker_status === "orphaned";

    if (hasApprovalMessage && !awaitingApproval) {
      this.approvalResolved.add(task.task_id);
      return;
    }

    this.approvalResolved.delete(task.task_id);
  }

  private async cleanupTaskState(taskId: string): Promise<void> {
    this.trackedTasks.delete(taskId);
    this.orbitEvents.untrackTask(taskId);
    this.approvalMessageTsByTask.delete(taskId);
    this.approvalInFlight.delete(taskId);
    this.approvalResolved.delete(taskId);
  }
}

export default SlackInterface;
