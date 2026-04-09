# Hosted Autonomous Engineering Implementation Plan

This plan turns Orbit into a split-brain system:

- the `orbit` CLI remains local-first and operates on the user's current working directory
- the hosted server runs remotely or locally, listens to Slack and external systems, provisions worker containers, clones repositories into those workers, coordinates multiple AI agents, and drives the full GitHub cycle through branch, test, push, and pull request flows

The plan is grounded in the current workspace:

- `crates/server` already provides the hosted control plane, task APIs, approvals, runtime inspection, reconciliation, and event streaming
- `crates/events` already provides shared typed task and connector event contracts
- `crates/orchestrator` already provides work-item and lane planning primitives
- `crates/tools` already provides hosted-agent launch and completion hooks
- `crates/runtime` already provides task/session/permission/runtime primitives
- `crates/webhooks` already provides inbound webhook infrastructure
- `extensions/orbit-slack` already provides the Slack operator connector

What is still missing is the real hosted execution system:

- server-managed worker containers
- repo clone and branch lifecycle inside those containers
- multi-agent decomposition for plan/implement/review/test/release work
- model routing per agent role and task shape
- outbound GitHub automation for commits, pushes, PRs, checks, and review updates

## 1. Product Boundaries

### Local CLI

The CLI must keep working against the user's current working directory.

- `orbit prompt ...` and related local commands continue to use the local filesystem
- hosted features are explicit, not implicit
- the CLI becomes an operator/admin client for the hosted system, not the only execution engine
- local hosted development must also be supported, using Docker-backed worker containers on the local machine

### Hosted System

The hosted system owns remote automation.

- receives work from Slack, webhooks, API, MCP-backed systems, and future schedulers
- plans the work into multiple lanes and agents
- provisions a worker container with the target repository mounted or cloned
- executes the task in an isolated repo workspace
- validates the result
- commits, pushes, opens or updates a PR, and reports status back to Slack/GitHub
- keeps humans in the loop through Slack approval and escalation flows

## 2. Target End-to-End Flow

1. A task arrives from Slack, API, GitHub webhook, or an MCP-backed system such as Linear.
2. `crates/server` persists the task and emits `task.created`.
3. `crates/orchestrator` expands the task into a plan:
   - planner lane
   - implementer lane
   - verifier/test lane
   - reviewer lane
   - release lane if GitHub output is enabled
4. The model router selects the best AI model for each lane based on task type, repo policy, cost, latency, and risk.
5. The execution manager provisions one or more worker containers for the repo and revision.
5. The repo lifecycle service clones the repository, checks out the base ref, creates a task branch, and prepares credentials.
6. Hosted agents run inside those workers against the checked-out repo, with parallel lanes where dependency rules allow it.
7. Slack remains the operator surface for progress, approvals, escalations, and release decisions.
8. Validation runs:
   - repo-specific test command set
   - lint/format
   - static checks
   - diff policy checks
9. If validation passes:
   - commit changes
   - push branch
   - create or update pull request
   - post task and PR state back into Orbit and Slack
10. If validation fails:
   - retry or route to a repair lane
   - or request approval/escalation depending on policy
11. Webhooks, MCP updates, and polling reconcile external state until the task reaches terminal state.

## 3. Architecture Changes

### 3.1 Control Plane

Keep `crates/server` as the source of truth.

Extend it with:

- repository execution requests:
  - repo URL
  - installation/credential context
  - base branch or commit
  - execution policy
- task output records:
  - branch name
  - head SHA
  - PR number and URL metadata
  - validation summary
  - check run summary
- worker lease records:
  - worker id
  - container id
  - repo checkout path
  - execution heartbeat

New APIs to add:

- `POST /v1/tasks/:task_id/launch`
- `GET /v1/tasks/:task_id/repository`
- `GET /v1/tasks/:task_id/validation`
- `GET /v1/tasks/:task_id/github`
- `POST /v1/tasks/:task_id/retry`
- `POST /v1/tasks/:task_id/release`

### 3.2 Event Contract

Extend `crates/events` for repo and GitHub lifecycle visibility.

Add typed events for:

- `repo.clone.started`
- `repo.clone.completed`
- `repo.branch.created`
- `validation.started`
- `validation.failed`
- `validation.passed`
- `git.push.started`
- `git.push.failed`
- `pr.opened`
- `pr.updated`
- `pr.checks.requested`
- `pr.checks.completed`
- `review.requested`
- `review.completed`
- `mcp.ticket.synced`
- `mcp.ticket.commented`
- `model.selected`

### 3.3 Execution Manager

Add a new crate:

- `crates/executor`

Responsibility:

- provision worker containers
- assign repo workspaces
- launch hosted agents inside a worker
- report heartbeats and terminal state back to `crates/server`
- support cancel, retry, and reattachment where possible
- support local Docker-backed execution for development and single-node deployments

Why a new crate:

- the current `LaneWorkerTransport` seam in `crates/server` is ready for a real backend
- container lifecycle and worker leasing should not live inside HTTP handlers

### 3.4 Repository Lifecycle Service

Add a new crate:

- `crates/repo`

Responsibility:

- clone/fetch/pull repository
- create task branch names
- reset workspace to known base SHA
- stage files
- commit changes
- push to origin
- collect diff metadata

This should be reusable by both the executor and any future local automation.

### 3.5 GitHub Integration

Add a new crate:

- `crates/github`

Responsibility:

- GitHub App auth or token auth
- repo metadata lookup
- branch protections and merge policy inspection
- PR create/update
- check run create/update
- issue/comment sync
- review request / review comment sync

Inbound GitHub events should continue to route through `crates/webhooks`.

### 3.6 MCP and External Work Systems

Add a new crate:

- `crates/connectors`

Responsibility:

- normalize external work items from webhook and MCP systems
- support systems such as Linear through MCP server integration
- map external issue/ticket identifiers to Orbit task ids
- post status, summaries, and comments back to external systems

This crate should sit above raw webhook and MCP plumbing and below the server control plane.

### 3.7 Multi-Agent Work Graph

Expand `crates/orchestrator` from simple lane assignment into a graph builder.

Supported lane roles:

- `planner`
- `implementer`
- `verifier`
- `reviewer`
- `triager`
- `release`
- `memory-writer`
- `ticket-sync`

Execution graph rules:

- planner can fan out implementer and verifier lanes
- planner can fan out multiple implementer lanes when the work can be partitioned safely
- implementer produces code changes
- verifier consumes branch state and validation output
- reviewer consumes diff, validation result, and policy
- release lane is unlocked only after verifier and reviewer success
- ticket-sync lanes can update Slack, Linear, or GitHub issue state without mutating code

### 3.8 Model Routing

Add a model-routing policy layer owned by the control plane.

Inputs:

- lane role
- repository policy
- task priority
- task complexity
- required context window
- expected tool usage
- cost and latency budget

Outputs:

- selected model provider
- selected model id
- fallback model list
- maximum concurrency for that lane type

Initial routing examples:

- planner:
  - high reasoning, larger context
- implementer:
  - strong coding model with tool use
- verifier:
  - cheaper high-throughput coding or reasoning model
- reviewer:
  - strong reasoning model with regression focus
- ticket-sync:
  - smaller low-cost model

## 4. Worker Container Model

Each hosted task runs in a worker container with:

- Orbit worker runtime
- git
- language-specific toolchains selected by image profile
- repo checkout directory
- ephemeral credentials for GitHub fetch/push
- task-scoped environment and secrets
- model selection and lane metadata injected by the control plane

### Container lifecycle

1. Allocate worker image profile from task metadata.
2. Start container.
3. Clone repo into `/workspace/repo`.
4. Check out base ref and create `orbit/<task-id>-<lane-role>` branch.
5. Run one or more agents against that repo.
6. Persist artifacts:
   - manifest
   - logs
   - diff summary
   - test results
7. Destroy or recycle container after terminal state.

### Image profiles

Start with a small set:

- `base`
- `node`
- `python`
- `rust`
- `polyglot`

Selection sources:

- repo metadata
- detected files
- explicit task policy

### Local deployment mode

The same hosted system must be runnable on a developer machine.

- `crates/server` runs locally
- `crates/executor` provisions Docker containers on the local Docker daemon
- Slack remains connected if credentials are configured
- webhooks and MCP connectors can point at the local instance for development and demos

This local mode should use the same APIs and event contracts as the remote deployment.

## 5. Multi-Agent Execution Plan

### Phase A: Single worker, multi-lane sequencing

Implement first:

- one worker container per task
- multiple logical lanes operating in the same checked-out repo
- serialized branch mutation with shared artifact state
- model routing per lane, even if execution is still mostly serialized

This is enough to ship:

- planner -> implementer -> verifier -> reviewer

### Phase B: Parallel workers for independent lanes

Add later:

- isolated workers for review-only or verification-only lanes
- patch transport between lanes
- branch or worktree snapshots per lane
- parallel implementer lanes for safe disjoint file sets
- parallel verifier lanes for multi-command validation

### Agent prompts and artifacts

Each lane should produce structured outputs:

- planner:
  - execution plan
  - repo-aware task breakdown
  - commands to validate
- implementer:
  - code diff
  - changed file list
  - self-reported risk summary
- verifier:
  - command results
  - failing tests
  - pass/fail decision
- reviewer:
  - regression findings
  - merge recommendation

## 6. Validation Model

Validation must be first-class, not an afterthought.

### Validation sources

- repo-declared commands:
  - test
  - lint
  - typecheck
  - format-check
- Orbit policy defaults
- language-specific fallbacks

### Validation record

Persist per task:

- commands run
- exit codes
- stdout/stderr artifact references
- elapsed time
- pass/fail summary
- blocking findings

### Human-in-the-loop policy

Validation and release should always be able to hand control back to Slack.

Approval triggers:

- orphaned executor
- failing validation override
- risky diff or policy violation
- push blocked by branch protection
- PR ready-for-review confirmation
- merge or release approval

### Acceptance gate

No PR creation without one of:

- policy says draft PR allowed on failing validation
- explicit human approval
- validation passed

## 7. GitHub Cycle

### Outbound

After successful implementation or reviewable draft:

1. create task branch
2. commit with deterministic message format
3. push branch
4. create or update PR
5. attach summary comment with:
   - task intent
   - files changed
   - validation result
   - remaining risks

### Inbound

Receive via `crates/webhooks`:

- pull request opened/synchronize/reopened/closed
- issue comment created
- review submitted
- check suite/check run updates
- push events

Map inbound GitHub events back to tasks by:

- task branch name
- PR metadata
- stored external ids in task context

### External system cycle

Support the same round-trip for non-GitHub systems where possible.

- Linear issue or project state can enter Orbit through MCP
- Orbit can post execution summaries, validation failures, and PR links back to Linear
- Slack remains the human approval surface even when the originating task came from another system

## 8. Slack and Human Oversight

Slack remains the operator surface, not the source of truth.

Add to `extensions/orbit-slack`:

- task launch summary with repo and branch
- validation summary cards
- PR opened/updated messages
- explicit approve/retry/cancel/release actions
- review findings summaries
- model selection visibility per lane
- external ticket linkage summaries for systems such as Linear

Approval types to support:

- orphaned executor
- failing validation override
- draft PR allowed
- push blocked by policy
- merge/release approval
- external ticket state transition approval where required

## 9. Data Model Additions

Extend hosted task context with:

- `repository`
- `repo_provider`
- `repo_owner`
- `repo_name`
- `installation_id`
- `base_ref`
- `base_sha`
- `head_ref`
- `head_sha`
- `pull_request_number`
- `pull_request_url`
- `validation_status`
- `validation_summary`
- `worker_image_profile`
- `container_id`
- `artifact_root`

## 10. Security Model

The hosted system needs stricter boundaries than the CLI.

### Requirements

- no long-lived git credentials inside workers
- per-task short-lived GitHub credentials
- isolated containers per repo or trust boundary
- explicit allowed command policy for validation commands
- secret scoping by workspace/repository
- audit trail for every push, PR, and approval action
- separate MCP and webhook credentials from repo execution credentials

### Initial non-goals

- arbitrary shared host execution
- unrestricted shell on the server host
- multi-tenant repo mixing in the same worker container

## 11. Phased Delivery

### Phase 1: Repo-aware hosted tasks

Deliver:

- add repo metadata to task creation
- persist repo execution context in `crates/server`
- add `crates/repo` for clone/fetch/branch primitives
- add task events for repo lifecycle
- make local hosted mode work with Docker-backed workers

Exit criteria:

- server can create a task tied to a repository and base ref
- server can clone and prepare a branch in an isolated workspace
- server can do the same against a local Docker daemon

### Phase 2: Real container executor

Deliver:

- add `crates/executor`
- implement container-backed `LaneWorkerTransport`
- heartbeat and terminal callbacks
- artifact persistence
- Docker local backend first, remote backend second

Exit criteria:

- a hosted task runs inside a provisioned worker container
- cancel/reconcile/runtime inspection all work against real workers

### Phase 3: Planner and verifier lanes

Deliver:

- expand `crates/orchestrator` into multi-lane execution graph
- planner lane output schema
- verifier lane output schema
- validation record persistence
- model-routing policy per lane

Exit criteria:

- one task can automatically produce plan -> implementation -> verification flow
- each lane is assigned a model intentionally rather than using one global model

### Phase 4: GitHub outbound automation

Deliver:

- add `crates/github`
- branch push
- PR create/update
- check run reporting

Exit criteria:

- validated hosted task can push a branch and open/update a PR

### Phase 5: GitHub inbound loop

Deliver:

- connect `crates/webhooks` to task lookup and external-id reconciliation
- PR comment and review ingestion
- check result synchronization
- external task update hooks for Slack and future connectors

Exit criteria:

- external GitHub state can update hosted task state automatically

### Phase 6: Human oversight and release policies

Deliver:

- Slack actions for validation override, draft PR, release, retry, cancel
- policy-driven release gating
- review summary cards
- explicit HITL escalation paths from external systems back into Slack

Exit criteria:

- operator can supervise the full hosted workflow from Slack and CLI

### Phase 7: Parallel agent execution

Deliver:

- worktree or snapshot isolation per lane
- parallel verifier/reviewer workers
- patch merge and conflict strategy
- safe fan-out of multiple implementer agents
- repo mutation locks and merge policy

Exit criteria:

- multi-agent tasks can run concurrently without corrupting repo state

### Phase 8: MCP work management integrations

Deliver:

- `crates/connectors` normalization layer
- Linear-over-MCP intake and status sync
- external-id mapping between Linear tickets, Orbit tasks, Slack threads, and GitHub PRs

Exit criteria:

- a Linear task can become an Orbit task
- Orbit can post progress and PR linkage back to Linear
- Slack remains the HITL surface for approvals

## 12. Recommended Crate Ownership

- `crates/server`
  - control plane API
  - task persistence
  - approvals
  - event stream
- `crates/events`
  - shared hosted task, repo, validation, GitHub, and connector contracts
- `crates/orchestrator`
  - lane graph and policy-aware routing
- `crates/executor`
  - worker/container lifecycle
- `crates/repo`
  - repo clone/branch/commit/push
- `crates/github`
  - GitHub App/API integration
- `crates/connectors`
  - MCP and external work-system normalization
- `crates/webhooks`
  - inbound GitHub and external event ingestion
- `crates/tools`
  - agent process execution and hosted callbacks
- `extensions/orbit-slack`
  - operator UI and approvals

## 13. Immediate Next Steps

Build in this order:

1. Add repository execution context to hosted tasks in `crates/server` and `crates/events`.
2. Implement local Docker-backed execution in `crates/executor` and connect it to the existing `LaneWorkerTransport` seam.
3. Implement `crates/repo` with clone/fetch/branch/commit/push primitives.
4. Expand `crates/orchestrator` into planner/implementer/verifier/reviewer graph output plus per-lane model routing.
5. Add `crates/github` for PR/check run operations.
6. Connect `crates/webhooks` to GitHub task reconciliation.
7. Extend Slack for validation, approval, and PR lifecycle actions.
8. Add `crates/connectors` for MCP-backed systems such as Linear.

## 14. Definition of Done

The hosted autonomous engineering system is considered operational when:

- Slack can create a hosted task for a GitHub repo
- the same hosted stack can run locally using Docker-backed workers
- the server provisions a worker container and clones that repo
- Orbit automatically runs planner, implementer, verifier, and reviewer agents
- the system can route models per lane and task type
- the system can fan out multiple AI agents in parallel where safe
- validation results are persisted and emitted over the event stream
- the system can commit, push, and open or update a PR automatically
- GitHub webhooks reconcile PR and check state back into Orbit
- humans can approve, retry, cancel, or release from Slack or CLI
- external systems such as Linear can create or synchronize tasks through MCP or webhook intake
- the local CLI still works purely against the current working directory without depending on hosted repo execution
