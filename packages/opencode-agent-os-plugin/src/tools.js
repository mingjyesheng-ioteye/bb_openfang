import { tool } from "@opencode-ai/plugin"

export function createTools(runtime) {
  return {
    memory_kv_set: tool({
      description: "Store a JSON value in Agent OS memory KV",
      args: {
        agentId: tool.schema.string().describe("Agent identity key"),
        key: tool.schema.string().describe("Memory key"),
        value: tool.schema.any().describe("JSON value to store"),
      },
      async execute(args) {
        runtime.memory.set(args.agentId, args.key, args.value)
        return JSON.stringify({ ok: true, key: args.key })
      },
    }),

    memory_kv_get: tool({
      description: "Read a JSON value from Agent OS memory KV",
      args: {
        agentId: tool.schema.string().describe("Agent identity key"),
        key: tool.schema.string().describe("Memory key"),
      },
      async execute(args) {
        const value = runtime.memory.get(args.agentId, args.key)
        return JSON.stringify({ found: value !== null, value })
      },
    }),

    task_post: tool({
      description: "Create a task in the Agent OS task board",
      args: {
        title: tool.schema.string().describe("Task title"),
        description: tool.schema.string().describe("Task description"),
        assignee: tool.schema.string().optional().describe("Optional assignee profile"),
      },
      async execute(args) {
        const task = runtime.tasks.post({
          id: runtime.makeId(),
          title: args.title,
          description: args.description,
          assignee: args.assignee,
        })
        return JSON.stringify(task)
      },
    }),

    task_list: tool({
      description: "List tasks from the Agent OS task board",
      args: {
        status: tool.schema.string().optional().describe("Filter by status: pending, claimed, completed, blocked"),
        limit: tool.schema.number().int().positive().max(200).optional().describe("Max items to return"),
      },
      async execute(args) {
        const tasks = runtime.tasks.list({
          status: args.status,
          limit: args.limit,
        })
        return JSON.stringify({ tasks })
      },
    }),
  }
}
