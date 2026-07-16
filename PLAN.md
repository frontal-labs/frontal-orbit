# CLI Upgrade Plan: Closing the 2026 Gap

**Purpose:** This document is a directive for an engineering agent to adapt, improve, and extend our existing coding CLI. It identifies what the current market of AI coding tools gets wrong, what's structurally missing across the category, and a concrete, phased plan to fix it. Treat each section as a work order — implement, don't just discuss.

---

## 1. Diagnosis: What's broken across the category right now

Before writing code, internalize why competitors (Claude Code, Cursor, Codex, Copilot) frustrate users despite being state of the art. Every fix below traces back to one of these:

1. **Context is treated as a flat text buffer, not a structure.** Tools re-serialize the whole codebase into tokens every session. Result: ~90% of context window gets consumed by raw, uncompressed tool output (file dumps, log tails, search results) that the model doesn't need. This causes slow responses, high bills, and mid-task memory loss.
2. **Advertised context ≠ usable context.** Some tools claim 200K tokens but deliver 70-120K after internal truncation, causing multi-file work to silently degrade in quality.
3. **Rate limits cause context eviction, not just a pause.** Hitting a limit mid-session kills the working mental model; users restart rather than resume.
4. **Verification is bolted on, not built in.** Code "looks" correct (compiles, passes shallow tests) while containing logic errors, security holes, or hallucinated dependencies. AI-authored PRs show materially more critical/major bugs than human PRs.
5. **One model does everything.** Trivial mechanical edits and hard architectural decisions get routed through the same expensive frontier model, wasting tokens and money.
6. **Pricing is opaque and consumption-based with no cost controls.** Engineering teams can't forecast or control spend; bills spike 10-100x with no visibility into why.
7. **No persistent, queryable memory of the codebase.** Every session rediscovers structure from scratch instead of querying a durable representation.
8. **Security debt hides in "clean-looking" code.** Unsafe deserialization, missing input validation, weak crypto, hallucinated package names (dependency confusion) slip through because review is optional, not gated.
9. **Multi-agent setups suffer "prompt decay"** — long-lived system prompts and stale working memory silently degrade agent judgment over time.
10. **No first-party eval harness.** Every serious team ends up building their own evaluation loop because the tools don't ship one.

Any feature that doesn't map to fixing one of these ten items is not a priority for this cycle.

---

## 2. Target architecture

### 2.1 Codebase Graph Layer (replaces flat context stuffing)
- Build and maintain a persistent, incrementally-updated structural index per repo: AST, symbol table, dependency graph, call graph, test coverage map, semantic embeddings per node, and change history.
- Update incrementally on file save / git commit — never full re-index unless forced.
- Expose as a **query interface**, not a document dump: "callers of X," "files touched by last 5 commits," "tests covering this function," "diff since checkpoint Y."
- All agent context requests go through this layer first. Raw file contents are the last resort, not the default.
- Store server-side (or local index + optional cloud sync) so it survives session restarts and rate-limit evictions.

**Work items:**
- [ ] Design graph schema (nodes: files/functions/classes/tests/deps; edges: calls/imports/covers/modifies)
- [ ] Incremental indexer with file-watcher hooks
- [ ] Embedding store for semantic search over the graph
- [ ] Query API the agent calls instead of `read_file` / `grep` as first resort
- [ ] Migration path: fall back to raw file reads only when graph query returns insufficient signal

### 2.2 Model Router (replaces single-model-for-everything)
- Add a triage layer that classifies each task by complexity/risk *before* dispatching to a model.
- Cheap/fast local or small models handle: formatting, renames, boilerplate, doc comments, mechanical refactors.
- Frontier models reserved for: architectural decisions, ambiguous requirements, security-sensitive code, multi-file coordinated changes.
- Router decision should be logged and inspectable — user can see why a task was escalated or downgraded, and override it.

**Work items:**
- [ ] Complexity/risk classifier (heuristic first — file count touched, presence of security-sensitive patterns, ambiguity signals in the prompt; ML-based classifier later)
- [ ] Model registry with cost/latency/capability metadata per model
- [ ] Manual override flag (`--force-model=`) for power users
- [ ] Telemetry: cost saved vs. frontier-only baseline, shown to the user

### 2.3 Verification Pipeline (replaces "another LLM eyeballs it")
- Every proposed change passes through a **required**, deterministic gate before being shown as "done":
  - Type checker / linter
  - Compile or build check
  - Existing test suite + auto-generated property-based tests for touched functions
  - Static security scan (taint analysis, unsafe deserialization, weak crypto patterns, injection risk)
  - Dependency/package existence + license check (catches hallucinated packages before install)
- Failures loop back to the generating agent automatically with structured failure info (not just "test failed") — up to N auto-retry cycles before surfacing to the human.
- This pipeline is not optional or config-gated behind a flag nobody sets — it's default-on.

**Work items:**
- [ ] Pluggable verifier interface (language-specific backends: tsc/mypy/etc., pytest/jest runners, semgrep or similar for security)
- [ ] Auto-generated property-based tests for new/changed functions (contract-style: input/output invariants)
- [ ] Dependency existence + typosquatting check against real registries before any install command runs
- [ ] Retry loop with structured failure feedback (not raw stack traces — parsed, actionable)
- [ ] Human-visible "verification report" attached to every completed task

### 2.4 Checkpointed Memory (replaces monolithic growing thread)
- Replace ever-growing conversation history with explicit checkpoints at meaningful boundaries: passing test suite, merged diff, user-approved milestone.
- Sessions resume from last checkpoint + graph state, not from a raw transcript replay.
- Rate-limit hits or crashes become non-events — no lost context, no restart-from-scratch.

**Work items:**
- [ ] Checkpoint data model (graph snapshot ref + task state + verification results)
- [ ] Auto-checkpoint triggers (test pass, commit, explicit user save)
- [ ] Resume command that rebuilds working context from checkpoint, not transcript
- [ ] Prune/compact old conversational history aggressively; graph is the source of truth, not chat log

### 2.5 Cost transparency & control
- Real-time, itemized token/cost dashboard per task, per session, per day — visible before and after, not just on the invoice.
- Budget caps configurable per project/session with hard stop or warning thresholds.
- Show *why* a task cost what it did (which model tier, how many tool calls, graph-query hits vs. raw file reads).
- Long-term: move toward outcome-based pricing option (cost per verified merged change) alongside metered pricing, so efficient usage is rewarded rather than incidentally increasing vendor revenue.

**Work items:**
- [ ] Per-task cost/token instrumentation surfaced in CLI output
- [ ] Budget cap config + enforcement (soft warning + hard stop modes)
- [ ] Session-level and daily cost summary command
- [ ] Cost-efficiency scorecard comparing graph-query vs. raw-context usage

### 2.6 Security-by-default
- Make the security scan step in 2.3 non-skippable for anything touching auth, data access, deserialization, or network I/O.
- Maintain an allowlist of verified internal/approved packages; anything outside it triggers a warning + manual confirm.
- Log and surface CVE-relevant patterns even if the task didn't ask for a security review.

**Work items:**
- [ ] Security-sensitive code pattern detector (auth, crypto, deserialization, SQL/query construction, network calls)
- [ ] Escalation rule: security-sensitive changes always route to frontier model + full verification gate, regardless of router's complexity score
- [ ] Package allowlist + confirm-before-install for anything unrecognized

### 2.7 Eval harness (first-party, not left to the user)
- Ship a built-in way to define task-level evals (given input state + task, expected verification outcome) so teams can regression-test agent behavior on their own codebase, not just trust vendor benchmarks.
- Run relevant evals automatically after any change to prompts, routing rules, or model versions used internally.

**Work items:**
- [ ] Eval spec format (task description, repo fixture, pass/fail criteria tied to verifier output)
- [ ] CLI command to run eval suite against current config
- [ ] CI hook so eval suite runs on any change to the CLI's own prompts/config

---

## 3. Phased roadmap

**Phase 1 (Foundation — do first, everything else depends on it)**
- Codebase Graph Layer (2.1)
- Checkpointed Memory (2.4)
- Basic cost instrumentation (2.5, first two work items)

**Phase 2 (Trust & correctness)**
- Verification Pipeline (2.3)
- Security-by-default (2.6)

**Phase 3 (Efficiency & economics)**
- Model Router (2.2)
- Full cost transparency + budget caps (2.5 remainder)

**Phase 4 (Rigor & differentiation)**
- Eval harness (2.7)
- Outcome-based pricing option
- Public benchmarking against Claude Code / Cursor / Codex on token efficiency and verified-change accuracy, not raw SWE-bench score

---

## 4. Success metrics (what "better than all of them" means, concretely)

- **Token efficiency:** tokens consumed per verified, merged change — target beating current best-in-class reported ratios (competitors report ~5.5x variance between best and worst tools on the same task).
- **Bug rate:** critical/major issues per PR, measured against the verification pipeline's own gate — target below current human baseline, not just below AI baseline.
- **Session continuity:** zero lost work on rate-limit or crash events (measured via checkpoint-resume success rate).
- **Cost predictability:** user can forecast monthly spend within a stated margin before running a large task, not just after the invoice.
- **Security debt:** percentage of merged changes with zero flagged security-sensitive patterns pre-merge, trending toward 100%.

---

## 5. Explicit non-goals for this cycle

- Do not chase "bigger context window" as a headline number — chase *effective* usable context via the graph layer instead.
- Do not add more autonomous "walk away and come back" demos before the verification pipeline is solid — trust is built by narrower, verified wins, not flashier autonomy.
- Do not build a new foundation model. This plan assumes swappable frontier models via the router; the differentiation is architecture, not model training.

---

**Instruction to agent:** Implement in the phase order above. After each phase, run the eval harness (once built) or, until then, a manual regression pass on a representative multi-file repo, and report token cost, bug count, and time-to-completion deltas against the current CLI baseline before proceeding to the next phase.
