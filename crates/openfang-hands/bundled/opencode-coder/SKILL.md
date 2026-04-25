---
name: opencode-api
description: "Drive the bb_opencode coding agent via its REST API on port 4201 — sessions, messages, file ops, LSP"
---
# bb_opencode REST API Reference

bb_opencode is a full AI coding agent running as a sidecar on `http://127.0.0.1:4201`. It provides deep code
intelligence: LSP, tree-sitter parsing, file read/write/edit, shell execution, web search, and 30+ LLM providers.

You drive it by creating sessions and sending messages. The agent handles all tool execution internally.

## Authentication

No auth required (loopback-only, auth disabled in BrainBook config).

## Directory Context

All instance-scoped requests require the project directory. Pass it as a header:
```
X-Opencode-Directory: /path/to/project
```
Or as a query param: `?directory=/path/to/project`

---

## Core Workflow

### Step 1 — Health Check
```
GET http://127.0.0.1:4201/global/health
→ { "healthy": true, "version": "1.x.x" }
```

### Step 2 — Create a Session
```
POST http://127.0.0.1:4201/session
Content-Type: application/json
X-Opencode-Directory: /path/to/project

{ "title": "brief description of the task" }

→ { "id": "ses_abc123", "title": "...", ... }
```

### Step 3 — Send a Task (synchronous — blocks until done)
```
POST http://127.0.0.1:4201/session/ses_abc123/message
Content-Type: application/json
X-Opencode-Directory: /path/to/project

{
  "parts": [{ "type": "text", "text": "your coding task" }],
  "agent": "build"
}

→ {
    "info": { "id": "msg_xxx", "role": "assistant", ... },
    "parts": [ ...tool calls, text, edits... ]
  }
```

### Step 4 — Fire-and-Forget (for long tasks)
```
POST http://127.0.0.1:4201/session/ses_abc123/prompt_async
Content-Type: application/json
X-Opencode-Directory: /path/to/project

{ "parts": [{ "type": "text", "text": "your task" }] }

→ 204 No Content   (returns immediately)
```
Then poll for completion:
```
GET http://127.0.0.1:4201/session/ses_abc123/message
→ [ ...messages array, last one is the AI response ]
```

---

## Agent Types

Pass as `"agent"` field in the message body:

| Agent | Purpose |
|---|---|
| `"build"` | Default. Full access: read, write, edit, bash, web search |
| `"plan"` | Read-only analysis. No file edits. Good for exploration. |
| `"general"` | General-purpose subagent |
| `"explore"` | Read-only file and code exploration |

---

## Useful Supporting Endpoints

### Read messages from a session
```
GET http://127.0.0.1:4201/session/ses_abc123/message
→ [ { "id": "msg_xxx", "role": "user"|"assistant", "parts": [...] }, ... ]
```

### Cancel a running task
```
POST http://127.0.0.1:4201/session/ses_abc123/abort
→ 200 OK
```

### Get file diffs from the last assistant message
```
GET http://127.0.0.1:4201/session/ses_abc123/diff
→ { "files": [ { "path": "src/foo.ts", "diff": "..." }, ... ] }
```

### Read a file directly (bypass the agent)
```
GET http://127.0.0.1:4201/file/content?path=/absolute/path/to/file
X-Opencode-Directory: /path/to/project
→ { "content": "file content as string" }
```

### List available agents
```
GET http://127.0.0.1:4201/agent
→ [ { "name": "build", "description": "..." }, ... ]
```

### List available skills
```
GET http://127.0.0.1:4201/skill
→ [ { "name": "...", "description": "..." }, ... ]
```

### List all sessions
```
GET http://127.0.0.1:4201/session
X-Opencode-Directory: /path/to/project
→ [ { "id": "ses_xxx", "title": "...", ... }, ... ]
```

### Delete a session (cleanup)
```
DELETE http://127.0.0.1:4201/session/ses_abc123
→ 200 OK
```

---

## Example: Full End-to-End Coding Task

```
# 1. Check health
GET http://127.0.0.1:4201/global/health

# 2. Create session
POST http://127.0.0.1:4201/session
X-Opencode-Directory: C:/Users/mingj/Documents/GitHub/bb_openfang
{ "title": "Add error handling to scheduler" }
→ { "id": "ses_xyz" }

# 3. Explore first (plan agent — no edits)
POST http://127.0.0.1:4201/session/ses_xyz/message
X-Opencode-Directory: C:/Users/mingj/Documents/GitHub/bb_openfang
{
  "parts": [{ "type": "text", "text": "Read crates/openfang-kernel/src/scheduler.rs and describe the error handling gaps" }],
  "agent": "plan"
}

# 4. Implement fix (build agent — can edit files)
POST http://127.0.0.1:4201/session/ses_xyz/message
X-Opencode-Directory: C:/Users/mingj/Documents/GitHub/bb_openfang
{
  "parts": [{ "type": "text", "text": "Now add proper error propagation to the scheduler — wrap panicking unwrap() calls with Result, propagate errors to the caller" }],
  "agent": "build"
}

# 5. Verify diffs
GET http://127.0.0.1:4201/session/ses_xyz/diff

# 6. Cleanup
DELETE http://127.0.0.1:4201/session/ses_xyz
```

---

## Tips

- **Be specific** in task prompts: include exact file paths, function names, error messages, and line numbers.
- **Use "plan" before "build"** for non-trivial changes: explore and understand before editing.
- **Multi-turn sessions**: send follow-up messages to the same session to refine or extend work.
- **Project path matters**: always pass the correct `X-Opencode-Directory` — it scopes file access and context.
- **Long tasks**: use `prompt_async` + poll instead of the synchronous endpoint to avoid HTTP timeouts.
