# Orbit Server

Hosted control-plane services for Orbit tasks, lane execution, event streaming, approvals, and policy-driven recovery.

## Current Surface

The server currently provides:

- hosted task APIs for create, list, inspect, cancel, reconcile, approve, and complete flows
- a filterable WebSocket event stream at `/v1/events/ws`
- persisted task and event state for restart recovery
- hosted-agent runtime inspection and reconciliation from manifest artifacts
- orphaned hosted-agent policy with timed retry, approval, and cancel behavior
- a read-only orphan policy inspection endpoint at `/v1/policies/orphans`

## Key Routes

- `GET /health`
- `GET /v1/status`
- `GET /v1/version`
- `GET /v1/policies/orphans`
- `GET /v1/tasks`
- `POST /v1/tasks`
- `GET /v1/tasks/:task_id`
- `POST /v1/tasks/:task_id/context`
- `GET /v1/tasks/:task_id/runtime`
- `POST /v1/tasks/:task_id/cancel`
- `POST /v1/tasks/:task_id/approval`
- `POST /v1/tasks/:task_id/reconcile`
- `POST /v1/tasks/:task_id/complete`
- `GET /v1/events/ws`
- `POST /v1/connectors/:connector/interactions`
- `POST /v1/connectors/:connector/events`

`GET /v1/tasks` accepts optional filters for `status`, `source`, `user_id`, `channel_id`, `thread_ts`, `repository`, and `limit`. `status` may be provided as a comma-separated list such as `pending,running`.

The event stream accepts optional query filters:

- `task_id`
- `lane_id`
- `topic`
- `event`
- `status`
- `source`
- `user_id`
- `channel_id`
- `thread_ts`
- `repository`
- `limit`

For connector use cases, `task_id`, `lane_id`, `source`, `user_id`, `channel_id`, `thread_ts`, and `repository` may be provided as comma-separated lists.

## Orphan Policy

The control plane can apply timed policy to orphaned hosted lanes when a persisted hosted-agent manifest is still non-terminal but no live control is attached.

Global policy environment variables:

- `ORBIT_SERVER_ORPHAN_APPROVAL_DELAY_SECS`
- `ORBIT_SERVER_ORPHAN_AUTO_RETRY_SECS`
- `ORBIT_SERVER_ORPHAN_AUTO_CANCEL_SECS`
- `ORBIT_SERVER_ORPHAN_POLICY_RULES`

`ORBIT_SERVER_ORPHAN_POLICY_RULES` is a JSON array of ordered match rules. The first matching rule wins.

Example:

```json
[
  {
    "repository": "repo-fast-policy",
    "source": "api",
    "approval_delay_secs": 0,
    "auto_retry_after_secs": 30
  },
  {
    "repository": "repo-prod",
    "priority": "high",
    "approval_delay_secs": 0,
    "auto_cancel_after_secs": 900
  }
]
```

Inspect policy and preview the effective match with:

```bash
curl "http://127.0.0.1:8788/v1/policies/orphans"
curl "http://127.0.0.1:8788/v1/policies/orphans?repository=repo-fast-policy&source=api"
curl "http://127.0.0.1:8788/v1/policies/orphans?repository=repo-prod&priority=high"
```

## Development

Run the server tests with:

```bash
cargo test -p orbit-server
```

For a step-by-step operator runbook covering hosted task inspection, orphan recovery, approvals, and policy tuning, see `docs/hosted-task-operations.md`.
