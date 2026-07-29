import { resolvePluginConfig } from "./config.js"
import { createRuntime } from "./runtime.js"
import { createTools } from "./tools.js"
import { enforcePolicy } from "./policy.js"

function profileForAgent(config, agentName) {
  if (agentName && config.agents[agentName]) return config.agents[agentName]
  return config.agents.default || {}
}

function stringifyCompact(value) {
  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

export const AgentOsPlugin = async (pluginInput, options = {}) => {
  const config = resolvePluginConfig(options, pluginInput.worktree || pluginInput.directory)
  const runtime = createRuntime(config, pluginInput)

  await runtime.log("info", "agent os plugin initialized", {
    dbPath: config.dbAbsolutePath,
  })

  return {
    config: async () => {
      await runtime.log("debug", "config hook received")
    },

    tool: createTools(runtime),

    "chat.params": async (input, output) => {
      const profile = profileForAgent(runtime.config, input.agent)

      if (typeof profile.temperature === "number") output.temperature = profile.temperature
      if (typeof profile.topP === "number") output.topP = profile.topP
      if (typeof profile.topK === "number") output.topK = profile.topK
      if (typeof profile.maxOutputTokens === "number") output.maxOutputTokens = profile.maxOutputTokens

      output.options = output.options || {}
      output.options.agent_os_profile = input.agent || "default"
    },

    "tool.execute.before": async (input, output) => {
      enforcePolicy(input, output, runtime.config)
    },

    "tool.execute.after": async (input, output) => {
      const text = String(output.output || "")
      if (!text.trim()) return

      const limit = runtime.config.memory.maxObservationChars
      const content = text.length > limit ? text.slice(0, limit) : text

      runtime.memory.addObservation({
        id: runtime.makeId(),
        sessionId: input.sessionID,
        agentId: "shared",
        source: input.tool,
        content,
        createdAt: new Date().toISOString(),
      })
    },

    "experimental.chat.system.transform": async (input, output) => {
      const tasks = runtime.tasks.list({ status: "pending", limit: 5 })
      if (!tasks.length) return

      output.system.push("## Agent OS Active Tasks")
      for (const task of tasks) {
        output.system.push(`- [${task.status}] ${task.title}: ${task.description}`)
      }

      output.system.push("## Agent OS Rules")
      output.system.push("- Respect task status transitions and do not mark work complete without evidence.")
      output.system.push("- Avoid blocked command/path patterns defined by policy.")
      output.system.push(`- Current model profile: ${stringifyCompact(input.model)}`)
    },

    event: async ({ event }) => {
      if (event.type === "session.created" || event.type === "session.error" || event.type === "session.compacted") {
        await runtime.log("info", "session lifecycle event", {
          eventType: event.type,
          sessionID: event.properties?.sessionID,
        })
      }
    },
  }
}
