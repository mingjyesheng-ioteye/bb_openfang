import path from "node:path"

const DEFAULTS = {
  dbPath: ".opencode/agent-os/agent-os.sqlite",
  memory: {
    maxObservationChars: 3000,
  },
  policy: {
    denyReadPatterns: [".env"],
    denyBashSubstrings: ["rm -rf /"],
  },
  agents: {
    default: {
      temperature: 0.2,
      topP: 1,
      topK: 0,
      maxOutputTokens: undefined,
    },
  },
}

function mergeShallow(base, input) {
  return {
    ...base,
    ...(input || {}),
  }
}

export function resolvePluginConfig(options, runtimeDirectory) {
  const merged = mergeShallow(DEFAULTS, options)
  merged.memory = mergeShallow(DEFAULTS.memory, options?.memory)
  merged.policy = mergeShallow(DEFAULTS.policy, options?.policy)
  merged.agents = mergeShallow(DEFAULTS.agents, options?.agents)

  const dbPath = typeof merged.dbPath === "string" && merged.dbPath.trim() ? merged.dbPath : DEFAULTS.dbPath
  merged.dbAbsolutePath = path.isAbsolute(dbPath) ? dbPath : path.join(runtimeDirectory, dbPath)

  return merged
}
