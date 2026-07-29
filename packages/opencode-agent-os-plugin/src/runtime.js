import crypto from "node:crypto"
import { openDatabase } from "./db.js"
import { createMemoryService } from "./memory-service.js"
import { createTaskService } from "./task-service.js"

export function createRuntime(config, pluginInput) {
  const db = openDatabase(config.dbAbsolutePath)
  const memory = createMemoryService(db)
  const tasks = createTaskService(db)

  async function log(level, message, extra = {}) {
    try {
      await pluginInput.client.app.log({
        body: {
          service: "openfang-agent-os-plugin",
          level,
          message,
          extra,
        },
      })
    } catch {
      // Do not fail plugin behavior if logging API is unavailable.
    }
  }

  function makeId() {
    return crypto.randomUUID()
  }

  return {
    config,
    db,
    memory,
    tasks,
    log,
    makeId,
  }
}
