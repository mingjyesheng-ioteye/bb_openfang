import fs from "node:fs"
import path from "node:path"
import Database from "better-sqlite3"

const MIGRATIONS = [
  `
CREATE TABLE IF NOT EXISTS ao_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
`,
  `
CREATE TABLE IF NOT EXISTS ao_memory_kv (
  agent_id TEXT NOT NULL,
  key TEXT NOT NULL,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (agent_id, key)
);
`,
  `
CREATE TABLE IF NOT EXISTS ao_tasks (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  status TEXT NOT NULL,
  assignee TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
`,
  `
CREATE TABLE IF NOT EXISTS ao_memory_fragments (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  source TEXT NOT NULL,
  content TEXT NOT NULL,
  created_at TEXT NOT NULL
);
`,
]

export function openDatabase(filePath) {
  const dir = path.dirname(filePath)
  fs.mkdirSync(dir, { recursive: true })

  const db = new Database(filePath)
  db.pragma("journal_mode = WAL")
  db.pragma("busy_timeout = 5000")

  runMigrations(db)
  return db
}

function runMigrations(db) {
  const now = new Date().toISOString()
  for (let i = 0; i < MIGRATIONS.length; i += 1) {
    db.exec(MIGRATIONS[i])
  }

  db.prepare(
    `INSERT INTO ao_meta (key, value, updated_at)
     VALUES (?, ?, ?)
     ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at`,
  ).run("schema_version", String(MIGRATIONS.length), now)
}
