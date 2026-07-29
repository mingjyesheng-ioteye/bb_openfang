export function createMemoryService(db) {
  const setStmt = db.prepare(
    `INSERT INTO ao_memory_kv (agent_id, key, value_json, updated_at)
     VALUES (?, ?, ?, ?)
     ON CONFLICT(agent_id, key)
     DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at`,
  )

  const getStmt = db.prepare(`SELECT value_json FROM ao_memory_kv WHERE agent_id = ? AND key = ?`)

  const addObservationStmt = db.prepare(
    `INSERT INTO ao_memory_fragments (id, session_id, agent_id, source, content, created_at)
     VALUES (?, ?, ?, ?, ?, ?)`,
  )

  return {
    set(agentId, key, value) {
      const now = new Date().toISOString()
      setStmt.run(agentId, key, JSON.stringify(value), now)
    },

    get(agentId, key) {
      const row = getStmt.get(agentId, key)
      if (!row) return null
      try {
        return JSON.parse(row.value_json)
      } catch {
        return row.value_json
      }
    },

    addObservation(input) {
      addObservationStmt.run(
        input.id,
        input.sessionId,
        input.agentId,
        input.source,
        input.content,
        input.createdAt,
      )
    },
  }
}
