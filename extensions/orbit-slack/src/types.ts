import type {
  OrbitGeneratedAppliedOrphanPolicy,
  OrbitGeneratedConnectorEventPayload,
  OrbitGeneratedEventEnvelope,
  OrbitGeneratedEventName,
  OrbitGeneratedEventPayload,
  OrbitGeneratedEventStatus,
  OrbitGeneratedEventTopic,
  OrbitGeneratedHostedTaskEventSummary,
} from "./generated/orbit-events";

// ============================================================================
// SLACK TYPES
// ============================================================================

export interface SlackUser {
  id: string;
  name: string;
  real_name: string;
  email?: string;
}

export interface SlackChannel {
  id: string;
  name: string;
  is_private: boolean;
}

export interface SlackMessage {
  user: string;
  channel: string;
  text: string;
  ts: string;
  thread_ts?: string;
  team?: string;
}

export interface SlackCommand {
  token: string;
  team_id: string;
  team_domain: string;
  channel_id: string;
  channel_name: string;
  user_id: string;
  user_name: string;
  command: string;
  text: string;
  response_url: string;
  trigger_id: string;
}

export interface SlackInteraction {
  type: string;
  token: string;
  action_ts: string;
  team: {
    id: string;
    domain: string;
  };
  user: {
    id: string;
    name: string;
  };
  channel: {
    id: string;
    name: string;
  };
  actions: Array<{
    name: string;
    type: string;
    value?: string;
  }>;
}

export interface SlackBlock {
  type: string;
  text?: {
    type: string;
    text: string;
    emoji?: boolean;
  };
  accessory?: SlackBlockAccessory;
  elements?: SlackBlockElement[];
}

export interface SlackBlockAccessory {
  type: string;
  [key: string]: unknown;
}

export interface SlackBlockElement {
  type: string;
  [key: string]: unknown;
}

export interface SlackAttachment {
  [key: string]: unknown;
}

export interface SlackBody {
  user: {
    id: string;
  };
  channel: {
    id: string;
  };
  message: {
    ts: string;
  };
  [key: string]: unknown;
}

export interface SlackEvent {
  type: string;
  user?: string;
  [key: string]: unknown;
}

// More permissive event type for Slack's complex event objects
export type SlackEventGeneric = {
  type: string;
  user?: string;
  [key: string]: unknown;
};

export interface SlackMessageOptions {
  channel: string;
  text?: string;
  blocks?: SlackBlock[];
  thread_ts?: string;
  attachments?: SlackAttachment[];
}

export interface SlackConversationContext {
  channel_id: string;
  thread_ts?: string;
  user_id: string;
  context: {
    current_task?: string;
    repository?: string;
    branch?: string;
    last_command?: string;
    preferences: Record<string, unknown>;
  };
  created_at: Date;
  updated_at: Date;
}

// ============================================================================
// ORBIT TYPES
// ============================================================================

export interface OrbitPromptRequest {
  prompt: string;
  model?: string;
  provider?: string;
  permission_mode?: string;
  allowed_tools?: string[];
}

export interface OrbitCliRequest {
  args: string[];
  force_json_output?: boolean;
}

export interface OrbitCliResponse {
  ok: boolean;
  exit_code?: number;
  args: string[];
  duration_ms: number;
  stdout: string;
  stderr: string;
  json?: Record<string, unknown>;
}

export interface OrbitTask {
  task_id: string;
  prompt: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  created_at: number;
  updated_at: number;
  description?: string;
  result?: string;
  error?: string;
  lane_id?: string;
  source?: string;
  user_id?: string;
  channel_id?: string;
  thread_ts?: string;
  approval_message_ts?: string;
  orphan_policy?: OrbitAppliedOrphanPolicy;
  repository?: string;
  repo_url?: string;
  base_ref?: string;
  branch?: string;
  execution_backend?: string;
  priority?: string;
  plan_id?: string;
  plan_kind?: string;
  work_item_id?: string;
  worker_id?: string;
  worker_status?: string;
}

export interface OrbitCreateTaskRequest {
  prompt: string;
  repository?: string;
  repo_url?: string;
  base_ref?: string;
  branch?: string;
  model?: string;
  provider?: string;
  permission_mode?: string;
  allowed_tools?: string[];
  priority?: "low" | "medium" | "high";
  source?: string;
  user_id?: string;
  channel_id?: string;
  thread_ts?: string;
}

export interface OrbitCreateTaskResponse {
  task_id: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  message: string;
  lane_id?: string;
  plan_kind?: string;
  worker_id?: string;
  worker_status?: string;
}

export interface OrbitListTasksQuery {
  status?: OrbitTask["status"] | string;
  source?: string;
  user_id?: string;
  channel_id?: string;
  thread_ts?: string;
  repository?: string;
  limit?: number;
}

export interface OrbitOrphanPolicyQuery {
  repository?: string;
  source?: string;
  priority?: string;
}

export interface OrbitOrphanPolicyRule {
  repository?: string;
  source?: string;
  priority?: string;
  approval_delay_secs?: number;
  auto_retry_after_secs?: number;
  auto_cancel_after_secs?: number;
}

export interface OrbitOrphanPolicyResponse {
  preview?: OrbitOrphanPolicyQuery;
  default_policy: OrbitAppliedOrphanPolicy;
  effective_policy: OrbitAppliedOrphanPolicy;
  configured_rules: OrbitOrphanPolicyRule[];
}

export interface OrbitUpdateTaskContextRequest {
  taskId: string;
  source?: string;
  user_id?: string;
  channel_id?: string;
  thread_ts?: string;
  approval_message_ts?: string;
}

export type OrbitEventTopic = OrbitGeneratedEventTopic;

export type OrbitEventStatus = OrbitGeneratedEventStatus;

export type OrbitEventName = OrbitGeneratedEventName;

export type OrbitEventEnvelope = OrbitGeneratedEventEnvelope;

export interface OrbitTaskRoutedEventPayload {
  lane_count?: number;
}

export interface OrbitLaneSignalEventPayload {
  role?: string;
  description?: string;
  detail?: string;
  transport?: Record<string, unknown>;
}

export interface OrbitApprovalRequestedEventPayload {
  approval_kind?: OrbitApprovalKind | string;
  reason?: string;
  detail?: string;
}

export interface OrbitApprovalResolvedEventPayload {
  approval_kind?: OrbitApprovalKind | string;
  action?: OrbitApprovalAction | string;
  resolved_by?: string;
  reason?: string;
}

export interface OrbitTerminalEventPayload {
  finish_reason?: string;
  tokens_output?: number;
  result?: string;
  error?: string;
  detail?: string;
  reconciled?: boolean;
  derived_state?: string;
}

export type OrbitEventTaskSummary = OrbitGeneratedHostedTaskEventSummary & {
  task_status?: OrbitTask["status"];
  result?: string;
  error?: string;
};

export type OrbitAppliedOrphanPolicy = OrbitGeneratedAppliedOrphanPolicy;

export type OrbitConnectorEventPayload = OrbitGeneratedConnectorEventPayload;

export type OrbitEventPayload = OrbitGeneratedEventPayload;

export interface OrbitEventStreamQuery {
  task_id?: string;
  lane_id?: string;
  topic?: OrbitEventTopic | string;
  event?: OrbitEventName | string;
  status?: OrbitEventStatus | string;
  source?: string;
  user_id?: string;
  channel_id?: string;
  thread_ts?: string;
  repository?: string;
  limit?: number;
}

export interface OrbitTrackedTask {
  taskId: string;
  channelId: string;
  threadTs?: string;
  userId?: string;
}

export interface OrbitStatusResponse {
  system: {
    status: "healthy" | "degraded" | "down";
    version: string;
    uptime: number;
  };
  tasks: {
    total_tasks: number;
    active_tasks: number;
    completed_tasks: number;
    failed_tasks: number;
  };
}

export interface OrbitSandboxResponse {
  status: "ready" | "busy" | "error";
  workspaces: number;
  active_sessions: number;
}

export type OrbitApprovalKind = "orphaned_hosted_agent";

export type OrbitApprovalAction = "cancel" | "retry";

export interface OrbitResolveApprovalRequest {
  taskId: string;
  approvalKind: OrbitApprovalKind;
  action: OrbitApprovalAction;
  resolvedBy?: string;
  reason?: string;
}

// ============================================================================
// SHARED TYPES (used by both Slack and Orbit)
// ============================================================================

export interface SlackTask {
  id: string;
  slack_task_id: string;
  orbit_task_id?: string;
  user_id: string;
  status:
    | "pending"
    | "submitted"
    | "running"
    | "completed"
    | "failed"
    | "cancelled";
  request: OrbitPromptRequest;
  response?: OrbitCliResponse;
  created_at: Date;
  updated_at: Date;
}

export interface SlackUserPreferences {
  default_model?: string;
  default_provider?: string;
  notification_level: "all" | "important" | "errors_only" | "none";
  auto_merge?: boolean;
}

export interface SlackUserPermissions {
  can_create_tasks: boolean;
  can_cancel_tasks: boolean;
  can_view_all_tasks: boolean;
  repositories?: string[];
}

export interface SlackUserSettings {
  id: string;
  slack_user_id: string;
  preferences: SlackUserPreferences;
  permissions: SlackUserPermissions;
  created_at: Date;
  updated_at: Date;
}

export interface TaskCreationRequest {
  prompt: string;
  repository?: string;
  branch?: string;
  model?: string;
  provider?: string;
  permission_mode?: string;
  allowed_tools?: string[];
  priority?: "low" | "medium" | "high";
}

export interface TaskProgressUpdate {
  task_id: string;
  status: string;
  message?: string;
  progress?: number;
  artifacts?: {
    type: "code" | "log" | "test_result" | "error";
    content: string;
    filename?: string;
  }[];
}
