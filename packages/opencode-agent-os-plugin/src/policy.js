import path from "node:path"

function looksLikeFileTool(toolName) {
  return toolName === "read" || toolName === "edit" || toolName === "write" || toolName === "apply_patch"
}

function extractPath(args) {
  if (!args || typeof args !== "object") return ""
  const candidates = [args.filePath, args.path, args.file]
  for (const item of candidates) {
    if (typeof item === "string") return item
  }
  return ""
}

export function enforcePolicy(input, output, config) {
  if (input.tool === "bash") {
    const command = String(output?.args?.command || "")
    for (const denied of config.policy.denyBashSubstrings) {
      if (denied && command.includes(denied)) {
        throw new Error(`Command blocked by policy: ${denied}`)
      }
    }
  }

  if (looksLikeFileTool(input.tool)) {
    const p = extractPath(output.args)
    const normalized = p ? p.split(path.sep).join("/") : ""
    for (const pattern of config.policy.denyReadPatterns) {
      if (pattern && normalized.includes(pattern)) {
        throw new Error(`Path blocked by policy pattern: ${pattern}`)
      }
    }
  }
}
