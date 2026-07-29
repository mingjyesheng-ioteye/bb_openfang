export function createTaskService(db) {
  const insertStmt = db.prepare(
    `INSERT INTO ao_tasks (id, title, description, status, assignee, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?)`,
  )

  const listStmt = db.prepare(
    `SELECT id, title, description, status, assignee, created_at, updated_at
     FROM ao_tasks
     WHERE (? IS NULL OR status = ?)
     ORDER BY created_at DESC
     LIMIT ?`,
  )

  return {
    post({ id, title, description, assignee }) {
      const now = new Date().toISOString()
      insertStmt.run(id, title, description, "pending", assignee || null, now, now)
      return {
        id,
        title,
        description,
        status: "pending",
        assignee: assignee || null,
        created_at: now,
        updated_at: now,
      }
    },

    list({ status, limit }) {
      const safeLimit = Number.isFinite(limit) ? Math.max(1, Math.min(200, Number(limit))) : 20
      return listStmt.all(status ?? null, status ?? null, safeLimit)
    },
  }
}
