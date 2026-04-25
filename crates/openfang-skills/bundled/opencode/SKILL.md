---
name: opencode
description: "Invoke the bundled opencode CLI binary via shell_exec to run coding tasks — sessions, file edits, LSP, bash, web search"
tags: [coding, opencode, typescript, rust, python, refactoring, debugging]
tools: [shell_exec]
runtime: prompt_only
---
# opencode CLI — Coding Agent

`opencode` is a full AI coding agent bundled as a sidecar binary. It runs inside a project directory and
provides deep code intelligence: LSP, tree-sitter, file read/write/edit, bash execution, web search, and
30+ LLM providers.

## Binary Location

In BrainBook, the binary is at:
```
C:/Users/mingj/AppData/Local/BrainBook/binaries/opencode/opencode.exe
```
Or use the alias `opencode` if it is on PATH.

## Running a Task (non-interactive)

Use `opencode run` to send a message to the coding agent and get output:

```bash
opencode run --dir "/path/to/project" --model "github-copilot/claude-sonnet-4.6" "your task here"
```

### Attaching to the Running Sidecar (port 4201)

In BrainBook, opencode is already running on port 4201. Attach to it instead of spawning a new process:

```bash
opencode run \
  --attach "http://127.0.0.1:4201" \
  --dir "/path/to/project" \
  --agent "build" \
  "your coding task here"
```

### Key Flags

| Flag | Description |
|---|---|
| `--attach <url>` | Attach to a running opencode server |
| `--dir <path>` | Project directory to run in |
| `--agent <name>` | Agent: `build` (default), `plan`, `explore`, `general` |
| `--model <p/m>` | Model in `provider/model` format |
| `--session <id>` | Continue an existing session |
| `--continue` | Continue the last session |
| `--format json` | Output raw JSON events |
| `--title <text>` | Title for the new session |

### Agent Types

| Agent | Purpose |
|---|---|
| `build` | Full access: read, write, edit, bash, web search (default) |
| `plan` | Read-only analysis, no file edits — good for exploration |
| `explore` | Read-only file and code exploration |
| `general` | General-purpose subagent |

## Recommended Workflow

### Step 1 — Explore First, Then Build

```bash
# Explore (no file edits)
opencode run \
  --attach "http://127.0.0.1:4201" \
  --dir "/path/to/project" \
  --agent "plan" \
  --title "explore: describe error handling in scheduler.rs" \
  "Read crates/openfang-kernel/src/scheduler.rs and describe the error handling gaps"

# Capture the session ID from the output, then continue:
opencode run \
  --attach "http://127.0.0.1:4201" \
  --dir "/path/to/project" \
  --agent "build" \
  --session "ses_xxx" \
  "Now fix the error handling: wrap all bare unwrap() calls with proper Result propagation"
```

### Step 2 — Check Available Sessions

```bash
opencode session list --attach "http://127.0.0.1:4201"
```

### Step 3 — Export Session Results

```bash
opencode export ses_xxx --attach "http://127.0.0.1:4201"
```

## Tips

- **Be specific**: include exact file paths, function names, and error messages in your task prompt.
- **Use `plan` before `build`** for non-trivial changes: explore first, implement second.
- **Continue sessions**: reuse `--session <id>` to send follow-up messages in the same context.
- **Long tasks**: `opencode run` blocks until the agent finishes — this is fine for most tasks.
- **Model selection**: `github-copilot/claude-sonnet-4.6` is available in BrainBook; pass as `--model`.

## Example: Full Coding Task

```bash
# 1. Start a planning session
opencode run \
  --attach "http://127.0.0.1:4201" \
  --dir "C:/Users/mingj/Documents/GitHub/bb_openfang" \
  --agent "plan" \
  --title "plan: add error handling to scheduler" \
  "Analyze crates/openfang-kernel/src/scheduler.rs — what error handling is missing?"

# 2. Implement the fix in the same session (use session ID from step 1 output)
opencode run \
  --attach "http://127.0.0.1:4201" \
  --dir "C:/Users/mingj/Documents/GitHub/bb_openfang" \
  --agent "build" \
  --session "ses_abc123" \
  "Add proper error propagation to the scheduler — replace unwrap() with Result, propagate errors up"

# 3. Verify with tests
opencode run \
  --attach "http://127.0.0.1:4201" \
  --dir "C:/Users/mingj/Documents/GitHub/bb_openfang" \
  --agent "build" \
  --session "ses_abc123" \
  "Run cargo test --workspace and fix any test failures"
```
