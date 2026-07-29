# OpenCode Agent OS Plugin Plan (Inspired by OpenFang)

## Goal
Build an OpenCode plugin that brings core Agent OS capabilities from OpenFang into OpenCode workflows:
- multi-agent orchestration
- memory substrate (session + long-term + shared)
- task board and delegation
- lifecycle hooks and policy guardrails
- skill and command packaging for repeatable execution

Reference implementation ideas can be borrowed from oh-my-openagent, but this plan targets a lean, OpenFang-aligned design first.

## Why This Fits OpenCode
OpenCode plugin APIs already provide the required surfaces:
- lifecycle and message hooks (`event`, `chat.message`, `chat.params`, `tool.execute.before`, `tool.execute.after`)
- system/context mutation hooks (`experimental.chat.system.transform`, `experimental.session.compacting`, `experimental.chat.messages.transform`)
- custom tools (`tool` map with `tool()` helper)
- plugin config loading (global, project, plugin directories, npm plugins)

This means we can ship most Agent OS features without forking OpenCode.

## OpenFang Capability Map -> OpenCode Plugin Capability

### 1) Agent Identity and Role Routing
OpenFang concept:
- explicit agent identity and role-oriented behavior
- model routing by complexity and role

Plugin implementation:
- maintain `agent profiles` in plugin config (planner, coder, reviewer, researcher)
- set model/temperature/options in `chat.params`
- inject role guardrails in `experimental.chat.system.transform`

### 2) Memory Substrate
OpenFang concept:
- structured KV, semantic recall, knowledge graph, sessions, usage tracking

Plugin implementation (phased):
- Phase 1: SQLite-backed structured memory + session memory + shared namespace
- Phase 2: semantic memory (embedding adapter + cosine retrieval)
- Phase 3: graph relations over entities (lightweight relation table first)

Where to hook:
- `chat.message`: capture candidate memory facts
- `tool.execute.after`: store tool-derived observations
- `experimental.session.compacting`: inject memory digest into compaction context

### 3) Task Board and Delegation
OpenFang concept:
- shared task queue with claim/complete lifecycle

Plugin implementation:
- plugin tools: `task_post`, `task_claim`, `task_complete`, `task_list`
- store tasks in SQLite with status machine (`pending`, `claimed`, `completed`, `blocked`)
- optional auto-post TODO candidates from message intent classifier

### 4) Tool and Policy Guardrails
OpenFang concept:
- before/after tool hooks and safety rails

Plugin implementation:
- `tool.execute.before`: policy checks, path guardrails, environment policy
- `tool.execute.after`: output shaping, metadata enrichment, memory extraction
- `permission.ask`: dynamic allow/deny based on task and policy profile

### 5) Skills and Command Packs
OpenFang concept:
- reusable skill blocks and structured prompts

Plugin implementation:
- skill pack directory in plugin config path
- load markdown skill specs and expose them via custom tools:
  - `skill_list`
  - `skill_run`
- optional slash command compatibility adapter inspired by oh-my-openagent plugin loader

## Target Architecture

### Runtime Components
1. Hook Orchestrator
- central dispatcher for plugin hook handlers
- deterministic execution order
- bounded timeouts and error isolation per hook

2. Agent Runtime
- profile resolver (agent -> model + behavior)
- run context assembler (session, memory digest, task status, policy)

3. Memory Service
- storage adapters:
  - sqlite structured store
  - optional vector provider adapter
- APIs for `remember`, `recall`, `kv_get`, `kv_set`, `entity_link`

4. Task Service
- queue operations and assignment logic
- relationship to session and agent profile

5. Policy Engine
- declarative rules for tool use, path access, command classes
- policy outcome: `allow`, `ask`, `deny`, `transform`

6. Skill Registry
- skill discovery and validation
- scoped execution through tools

## Storage Model (SQLite)
Tables (minimum):
- `ao_sessions` (session metadata)
- `ao_memory_kv` (agent_id, key, value_json)
- `ao_memory_fragments` (id, session_id, agent_id, source, content, tags, score)
- `ao_tasks` (id, title, description, status, assignee, created_at, updated_at)
- `ao_task_events` (task_id, event_type, payload_json, created_at)
- `ao_entities` and `ao_relations` (Phase 3)
- `ao_usage` (optional token/cost estimates)

Namespace strategy:
- per-agent namespace
- shared namespace for cross-agent coordination

## Plugin Hook Plan
- `config`: validate and normalize plugin config, initialize services
- `chat.message`: detect intent, update session/task context, schedule memory extraction
- `chat.params`: agent-specific model and sampling parameters
- `chat.headers`: optional trace and correlation headers
- `tool.execute.before`: enforce policy, sanitize risky arguments
- `tool.execute.after`: attach metadata, extract memories, update task state
- `experimental.chat.system.transform`: inject role, policy summary, active task, memory digest
- `experimental.session.compacting`: inject persistent state into compaction summaries
- `event`: react to `session.created`, `session.idle`, `session.error`, `session.compacted`

## Custom Tool Surface (MVP)
Memory tools:
- `memory_store`
- `memory_recall`
- `memory_kv_get`
- `memory_kv_set`

Task tools:
- `task_post`
- `task_claim`
- `task_complete`
- `task_list`

Coordination tools:
- `agent_route` (suggest next role/agent)
- `session_brief` (structured state summary)
- `skill_list`
- `skill_run`

## Phased Delivery Plan

### Phase 0: Bootstrap (1 week)
- plugin skeleton and config schema
- SQLite bootstrap and migrations
- structured logs and health checks

Exit criteria:
- plugin loads from project/global scopes
- DB initializes and migrations run cleanly

### Phase 1: Agent Context + Memory KV + Task Board (2 weeks)
- agent profile config and `chat.params` routing
- memory KV tools and session digest injection
- full task tool lifecycle
- baseline policy checks on tool execution

Exit criteria:
- can run a full coding loop with persistent task and memory state across messages

### Phase 2: Observation Memory + Compaction Intelligence (2 weeks)
- extract memories from message/tool outputs
- ranking and recall scoring
- compaction hook injects durable project state

Exit criteria:
- after long session compaction, agent resumes with correct task and memory continuity

### Phase 3: Delegation and Skill Packs (2 weeks)
- role-aware delegation heuristics
- skill registry, loading, and execution
- optional command compatibility layer

Exit criteria:
- user can run planner->coder->reviewer workflow with shared context

### Phase 4: Semantic and Graph Memory (optional, 2+ weeks)
- embedding adapter and vector recall
- lightweight entity-relation store

Exit criteria:
- semantic recall materially improves long-horizon task continuity

## Config Proposal
File name:
- `agent-os-plugin.jsonc`

Core keys:
- `agents`: role definitions and model overrides
- `memory`: backend, retention, extraction thresholds
- `tasks`: defaults, auto-claim strategy, stale detection
- `policy`: tool/path/network policies
- `skills`: paths and enable flags
- `telemetry`: logging and usage stats

## Test Strategy

### Unit
- memory ranking and extraction
- policy evaluation and argument sanitization
- task state transitions

### Integration
- hook order and side-effect correctness
- session continuity with compaction
- plugin load from global/project and npm specs

### Scenario tests
- long-running bugfix with compaction in middle
- multi-task queue with interruptions
- blocked command policy escalation path

## Risks and Mitigations

Risk: hook complexity causes brittle behavior
- Mitigation: strict phase gates, contract tests per hook, idempotent handlers

Risk: model/provider differences break routing assumptions
- Mitigation: explicit agent->provider/model schema and health checks

Risk: memory bloat or low-signal recall
- Mitigation: confidence thresholds, retention TTL, score decay, compaction summaries

Risk: plugin incompatibility across OpenCode versions
- Mitigation: version gate and compatibility mode flags (as seen in oh-my-openagent patterns)

## What to Reuse from oh-my-openagent
- plugin loader patterns for commands/skills/agents
- compatibility and diagnostics approach (`doctor`-style checks)
- conservative version-gating and fallback behavior

Do not copy behavior wholesale. Keep this plugin smaller and OpenFang-first:
- prioritize memory/task continuity over large hook count
- start with 8-12 high-value hooks, then expand
- keep feature flags for each subsystem

## First Implementation Backlog (Top 12)
1. Define config schema and defaults.
2. Build plugin bootstrap and service container.
3. Add SQLite migration runner.
4. Implement memory KV service and tools.
5. Implement task service and tools.
6. Add agent profile routing in `chat.params`.
7. Add system prompt injection in `experimental.chat.system.transform`.
8. Add pre/post tool policy hooks.
9. Add message/tool observation extraction.
10. Add compaction context injection.
11. Add health tool (`agent_os_health`).
12. Add integration tests for full loop continuity.

## Success Metrics
- continuity: resumed sessions preserve active task + key context after compaction
- safety: policy violations are caught before execution
- velocity: fewer manual restatements by user during long tasks
- reliability: plugin boot + migration success >= 99% in local test matrix
