# OpenFang Agent OS Plugin for OpenCode

This package provides a minimal Agent OS scaffold for OpenCode plugins:

- SQLite-backed memory KV store
- SQLite-backed task board (post/list)
- policy guardrails in `tool.execute.before`
- tool observation capture in `tool.execute.after`
- agent profile parameter tuning in `chat.params`

## Quick Start

Add to OpenCode plugin config:

```json
{
  "plugin": [
    ["@openfang/opencode-agent-os-plugin", {
      "dbPath": ".opencode/agent-os/agent-os.sqlite",
      "policy": {
        "denyReadPatterns": [".env"],
        "denyBashSubstrings": ["rm -rf /", "curl | sh"]
      }
    }]
  ]
}
```

## Exposed Tools

- `memory_kv_set`
- `memory_kv_get`
- `task_post`
- `task_list`

## Notes

- This is a Phase 0/1 scaffold focused on continuity and guardrails.
- Semantic memory, delegation and advanced graph memory are planned next.
