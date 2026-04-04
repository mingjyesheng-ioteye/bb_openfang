# bb_openfang Coding Capability Upgrade Plan

Date: 2026-04-02
Scope: Improve coding-agent performance by porting/adapting high-impact capabilities observed in claude-code-source-code-v2.1.88.

## 1) Outcome Targets

- Reduce tool-only shell fallback for code navigation by 60%+
- Increase first-pass code edit success rate by 30%+
- Reduce context bloat and loop latency on multi-file tasks
- Keep security posture at or above current baseline

## 2) Epic Breakdown

### EPIC A: Code Intelligence Toolkit
Goal: Give the model first-class search/navigation tools instead of shell-heavy workflows.

Deliverables:
- `file_search` (glob-based file discovery)
- `grep_search` (regex search with modes + pagination)
- `code_symbol_refs` (definition/references/hover; initial LSP-backed subset)

Primary integration files:
- `crates/openfang-types/src/tool.rs`
- `crates/openfang-runtime/src/tool_runner.rs`
- `crates/openfang-runtime/src/lib.rs`
- New module candidates:
  - `crates/openfang-runtime/src/code_search.rs`
  - `crates/openfang-runtime/src/code_intel.rs`

Acceptance criteria:
- Agent completes common codebase exploration tasks without `shell_exec` in >= 80% of cases
- `grep_search` supports `limit`, `offset`, and output modes (`content`, `files`, `count`)
- Result payloads are bounded to avoid runaway context growth

---

### EPIC B: Safe Edit Pipeline (Read-before-Write)
Goal: Prevent stale or blind edits and improve reliability of patch application.

Deliverables:
- Track read state per file (timestamp and/or content hash)
- Reject writes when file was never read in-session
- Reject writes when file changed after read
- Actionable error guidance (closest path suggestion, refresh instructions)

Primary integration files:
- `crates/openfang-runtime/src/agent_loop.rs`
- `crates/openfang-runtime/src/apply_patch.rs`
- `crates/openfang-runtime/src/tool_runner.rs`
- `crates/openfang-types/src/approval.rs` (policy tie-in if needed)

Acceptance criteria:
- All mutating file tools enforce read-before-write
- Stale-edit attempts are deterministically blocked
- Retry path is clear: read latest -> edit -> apply

---

### EPIC C: Shell Execution for Coding Loops
Goal: Make shell usage safer and more ergonomic for coding tasks.

Deliverables:
- Command classification (`read_only` vs `mutating`)
- Destructive command warnings (git/rm/database/infra patterns)
- Anti-stall validation (discourage long blocking sleeps)
- Better default guidance for background execution where relevant

Primary integration files:
- `crates/openfang-runtime/src/host_functions.rs`
- `crates/openfang-runtime/src/command_lane.rs`
- `crates/openfang-runtime/src/agent_loop.rs`

Acceptance criteria:
- Read-only commands can be parallelized safely
- Mutating commands stay serialized
- Warning coverage for high-risk command classes

---

### EPIC D: Plan Mode + Todo Control Loop
Goal: Improve quality on complex coding tasks with explicit planning and checklist tracking.

Deliverables:
- `enter_plan_mode` tool
- `exit_plan_mode` tool with user approval gate (configurable)
- `todo_write` tool and session-persisted task list

Primary integration files:
- `crates/openfang-runtime/src/prompt_builder.rs`
- `crates/openfang-runtime/src/agent_loop.rs`
- `crates/openfang-types/src/tool.rs`

Acceptance criteria:
- Complex tasks can be forced through planning phase
- Checklist is inspectable and updates over the session
- Exiting plan mode has explicit transition semantics

---

### EPIC E: Parallel Tool Orchestration
Goal: Improve throughput by concurrent execution of independent read-only tool calls.

Deliverables:
- Batch partitioning algorithm:
  - consecutive read-only calls => parallel batch
  - mutating or non-concurrency-safe calls => serialized
- Preserve deterministic ordering around mutating actions

Primary integration files:
- `crates/openfang-runtime/src/agent_loop.rs`
- `crates/openfang-runtime/src/command_lane.rs`

Acceptance criteria:
- Wall-clock latency drops on search-heavy tasks
- No race-related regressions on mutating workflows

---

### EPIC F: Prompt/Policy Upgrades for Coding Behavior
Goal: Teach the model to choose the right coding workflow by default.

Deliverables:
- Add system guidance:
  - parallelize independent reads
  - prefer search before full-file reads
  - enforce read-before-edit sequence
  - avoid claiming completion without tool execution

Primary integration files:
- `crates/openfang-runtime/src/prompt_builder.rs`

Acceptance criteria:
- Fewer redundant reads and less shell overuse
- Improved tool-choice consistency

---

### EPIC G: Test + Benchmark Harness
Goal: Validate correctness and quantify impact.

Deliverables:
- Unit tests for each new tool schema and validation
- Integration tests for orchestration order and stale-write blocking
- Scenario benchmarks for coding workflows (explore -> edit -> test loop)

Primary integration files:
- `crates/openfang-runtime/src/*` tests
- `crates/openfang-types/src/*` tests

Acceptance criteria:
- No regressions in existing security tests
- Benchmark deltas reported before/after

## 3) Sequenced Delivery Plan

### Wave 1 (Fastest Value, 1-2 weeks)
- EPIC A (partial): `file_search`, `grep_search`
- EPIC B (core): read-before-write enforcement
- EPIC F (minimal prompt guardrails)

### Wave 2 (Quality Lift, 1-2 weeks)
- EPIC D: plan mode + todo tools
- EPIC E: parallel read-only orchestration

### Wave 3 (Power User + Hardening, 1-2 weeks)
- EPIC A (advanced): `code_symbol_refs`
- EPIC C: shell classification/warnings
- EPIC G: benchmarks + hardening sweep

## 4) First PR Scope (Recommended Start)

PR Title:
- `feat(coding-core): add grep/file search tools + read-before-write guardrails`

In-scope changes:
1. Add tool definitions for:
   - `file_search`
   - `grep_search`
2. Implement execution handlers in runtime tool runner
3. Add bounded output controls (`limit`, `offset`, default caps)
4. Introduce read-state tracking in agent session runtime
5. Enforce read-before-write for file mutation path (`apply_patch` and/or file write flow)
6. Add tests:
   - tool schema parse/validation
   - grep pagination
   - stale write rejection

Out-of-scope for PR1:
- Full LSP integration
- Plan mode tooling
- Concurrency batching
- Advanced shell warning engine

Definition of done:
- All tests pass
- New tools appear in advertised tool list
- Mutating file operations fail with clear errors when read-state preconditions are unmet
- Basic docs added under `docs/` with examples

## 5) Implementation Task Board (Actionable)

Status legend: TODO | IN_PROGRESS | BLOCKED | DONE

### Track A: Tool Surface
- [DONE] A1. Add `file_search` schema to runtime tool definitions
- [DONE] A2. Add `grep_search` schema to runtime tool definitions
- [DONE] A3. Register tools in runtime tool registry
- [DONE] A4. Add docs/examples for both tools

### Track B: Runtime Execution
- [DONE] B1. Implement glob-backed file search handler
- [DONE] B2. Implement regex-backed grep handler
- [DONE] B3. Add response truncation/pagination bounds for search tools
- [DONE] B4. Add timing metrics hooks for search tools

### Track C: Edit Safety
- [DONE] C1. Add per-agent read-state structure
- [DONE] C2. Update file read path to record read metadata
- [DONE] C3. Add preflight check before mutation (`read-before-write`)
- [DONE] C4. Add stale file detection and error messaging

### Track D: Testing
- [DONE] D1. Unit tests for tool input validation
- [DONE] D2. Unit tests for pagination behavior
- [DONE] D3. Integration test coverage for read-before-write rejection
- [DONE] D4. Integration test for read->edit success path

### Track F: Parallel Orchestration (Wave 2)
- [DONE] F1. Add read-only batch partitioning in standard loop
- [DONE] F2. Add read-only batch partitioning in streaming loop
- [DONE] F3. Preserve ordered result emission across parallel batches
- [DONE] F4. Add deterministic concurrency test for parallel helper
- [DONE] F5. Add benchmark harness for latency improvement on search-heavy turns (synthetic + scenario workload)

### Track I: Plan Mode + Todo Control Loop (Wave 2)
- [DONE] I1. Add `enter_plan_mode` and `exit_plan_mode` tool schemas and runtime dispatch
- [DONE] I2. Add `todo_write` tool schema and runtime implementation with agent-scoped persisted checklist
- [DONE] I3. Add approval-policy gating documentation for `exit_plan_mode`
- [DONE] I4. Add runtime tests for plan mode toggling and todo mutation flows

### Track E: Rollout
- [DONE] E1. Feature flag for new coding tools (optional but recommended)
- [DONE] E2. Enable in canary profile
- [DONE] E3. Collect telemetry and compare against baseline
- [DONE] E4. Promote to default

### Track G: Shell Execution Hardening (Wave 3)
- [DONE] G1. Add shell command classification (`read_only` vs `mutating`) in shared runtime module
- [DONE] G2. Add non-blocking warnings for destructive shell commands
- [DONE] G3. Add anti-stall warnings for long blocking sleep/monitoring commands
- [DONE] G4. Integrate shell classification into agent-loop batch partitioning
- [DONE] G5. Add deterministic tests for classification, warning coverage, and shell-aware batching
- [DONE] G6. Add docs for shell warning behavior and background execution guidance

### Track H: Advanced Code Intelligence (Wave 3)
- [DONE] H1. Add `code_symbol_refs` tool schema and runtime dispatch
- [DONE] H2. Implement symbol definition/reference search baseline with pagination bounds
- [DONE] H3. Add compatibility aliases and read-only/coding tool classification updates
- [DONE] H4. Add runtime and type-layer tests for `code_symbol_refs`
- [DONE] H5. Upgrade `code_symbol_refs` to prefer MCP-exposed LSP symbol tools with automatic heuristic fallback

## 6) Risks and Mitigations

- Risk: Context blowup from unbounded search outputs
  - Mitigation: strict defaults + offset pagination + truncation markers
- Risk: False positives in stale-write checks during formatter hooks
  - Mitigation: allow explicit re-read fast path and informative error messages
- Risk: Added complexity in tool orchestration introduces regressions
  - Mitigation: keep PR1 serial execution; add parallelism only in Wave 2
- Risk: Model overuses new tools in suboptimal order
  - Mitigation: prompt heuristics in Wave 1 and tune after telemetry

## 7) Ownership Recommendation

- Runtime lead: EPIC A/B/E
- Policy/security lead: EPIC C/F
- Quality lead: EPIC G
- Docs/devex lead: examples, migration notes, CLI help updates

## 8) Success Metrics Dashboard

Track weekly:
- Avg tool calls per resolved coding task
- % tasks resolved without shell fallback
- First-pass patch success rate
- Median loop latency (coding scenarios)
- Error rate by tool (`grep_search`, `file_search`, mutating writes)

Targets after Wave 2:
- +30% first-pass patch success
- -25% median coding-task latency
- >=80% code exploration tasks resolved without shell fallback

## 9) Spec-Informed Gap Analysis (claurst/spec -> bb_openfang)

Reference baseline reviewed:
- `claurst/spec/03_tools.md`
- `claurst/spec/06_services_context_state.md`
- `claurst/spec/11_special_systems.md`

Current status note:
- Waves 1-3 are implemented in this repo, but the coder-agent roadmap can still be improved for operational quality and advanced workflows.

### Key Gaps and Priority

1. Task lifecycle depth (High)
- Gap: Current plan centers on `todo_write` checklisting, but lacks first-class task lifecycle APIs (`task_create`, `task_get`, `task_update`, `task_list`, `task_output`) for multi-step orchestration.
- Impact: Harder to coordinate long-running coding work, subagent output capture, and resumable flows.

2. Session memory lifecycle and compaction (High)
- Gap: No explicit coding-loop plan for memory compaction/summarization triggers and transcript budget recovery.
- Impact: Long sessions risk context bloat and lower-quality turns.

3. Permission ergonomics and policy explainability (High)
- Gap: Plan has safety controls but limited emphasis on user-facing deny/ask rationale consistency and policy introspection.
- Impact: Friction when tools are blocked; slower user trust calibration.

4. MCP resource ergonomics beyond tool invocation (Medium)
- Gap: `code_symbol_refs` MCP path exists, but roadmap does not explicitly cover MCP resource list/read and auth/diagnostics UX.
- Impact: Reduced discoverability and slower debugging of MCP-backed workflows.

5. Reliability telemetry for coding loops (Medium)
- Gap: Metrics are defined at dashboard level, but no explicit implementation track for per-tool error taxonomy and retry quality analytics.
- Impact: Harder to diagnose regressions and prove rollout quality.

6. Multi-agent orchestration maturity (Medium)
- Gap: Parallel read-only batching exists, but no explicit plan for structured parent/subagent task contracts and output aggregation.
- Impact: Limited scalability on large refactor/research tasks.

7. Long-running automation hooks (Low/Medium)
- Gap: No roadmap item for scheduled coding checks (cron-like periodic tasks) in local/dev workflows.
- Impact: Missed opportunity for continuous repo health automation.

## 10) Wave 4 (Coder Agent Quality+)

Goal: Upgrade coder-agent robustness and usability beyond core tooling by adding orchestration, memory hygiene, permission clarity, and MCP operability.

### EPIC J: Task Lifecycle V2
Deliverables:
- Add task APIs for create/get/update/list/output and task stop semantics
- Persist task state with clear ownership (`agent_id`, parent task, timestamps)
- Support resumable task execution after turn boundaries

Acceptance criteria:
- Complex coding jobs can be paused/resumed with no manual reconstruction
- Subagent outputs are queryable and auditable

### EPIC K: Session Memory + Context Compaction
Deliverables:
- Add compaction policy for long coding sessions (threshold-based + manual trigger)
- Add summary artifacts that preserve unresolved TODOs/open blockers
- Add post-compaction validation that critical constraints survive

Acceptance criteria:
- Measurable reduction in context size growth on long sessions
- No increase in unresolved-task loss after compaction

### EPIC L: Permission UX and Explainability
Deliverables:
- Standardize deny/ask reason schema across mutating and shell tools
- Add policy introspection tool output (what rule blocked/allowed this call)
- Add actionable remediation guidance in rejection payloads

Acceptance criteria:
- Users can identify and fix blocked-tool causes in one iteration
- Reduced repeated-denial loops in coding sessions

### EPIC M: MCP Operability Expansion
Deliverables:
- Add MCP resource list/read helpers in coding flows
- Add MCP auth/handshake diagnostics and startup health checks
- Add deterministic fallback chain docs for MCP outage scenarios

Acceptance criteria:
- MCP-backed coding tools are self-diagnosable without shell debugging
- Reduced time-to-resolution for MCP config issues

### EPIC N: Reliability and Benchmark Hardening
Deliverables:
- Add per-tool error codes and retry classification
- Record coding-loop SLOs (tool latency, fail rate, recovery success)
- Add benchmark scenarios for multi-agent and long-session compaction cases

Acceptance criteria:
- Regressions can be localized to tool/policy layer quickly
- Rollout decisions are data-backed at canary and default stages

## 11) Wave 4 Task Board (Actionable)

Status legend: TODO | IN_PROGRESS | BLOCKED | DONE

### Track J: Task Lifecycle V2
- [DONE] J1. Define task domain model and storage keys in runtime/kernel
- [DONE] J2. Implement `task_create`, `task_get`, `task_update`, `task_list`, `task_output`, `task_stop`
- [DONE] J3. Add schema/compat aliases and tool classification updates
- [DONE] J4. Add integration tests for resume and parent/subagent aggregation

### Track K: Memory + Compaction
- [DONE] K1. Add context-budget monitors and compaction trigger strategy
- [DONE] K2. Implement compaction summaries preserving TODOs/blockers/decisions
- [DONE] K3. Add manual compaction command/tool entrypoint
- [DONE] K4. Add regression tests for information retention post-compaction

### Track L: Permission Explainability
- [DONE] L1. Normalize permission decision payloads with machine-readable reason codes
- [DONE] L2. Add policy-inspection helper output for blocked tool calls
- [DONE] L3. Add docs for common deny paths and fast remediation
- [DONE] L4. Add tests ensuring consistent rejection messaging across tool classes

### Track M: MCP Operability
- [DONE] M1. Add MCP resource list/read tool adapters (where available)
- [DONE] M2. Add MCP connectivity/auth diagnostics at startup and per-call fallback hints
- [DONE] M3. Add deterministic tests for missing MCP tool/resource/auth paths
- [DONE] M4. Update configuration docs with validated Windows/macOS/Linux examples

### Track N: Reliability and Benchmarks
- [DONE] N1. Define coding-loop SLO metrics and emission points
- [DONE] N2. Add per-tool failure taxonomy and retry reason attribution
- [DONE] N3. Expand benchmark harness to include long-session compaction scenarios
- [DONE] N4. Add canary scorecard template for Wave 4 promotion decisions

## 12) Updated Success Targets (Post Wave 4)

- +45% first-pass patch success vs. pre-Wave-1 baseline
- -35% median coding-task latency on search/edit/test loops
- >=90% code exploration tasks resolved without shell fallback
- <3% repeated permission-denial loop rate per 100 coding turns
- <2% MCP-related hard-failure rate after fallback paths
