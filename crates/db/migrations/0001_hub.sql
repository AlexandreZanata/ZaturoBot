CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    request_json TEXT NOT NULL,
    state TEXT NOT NULL,
    target_worker TEXT NOT NULL,
    assigned_worker TEXT,
    priority INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    deadline_at INTEGER NOT NULL,
    lease_expires_at INTEGER,
    body TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS tasks_queue ON tasks(state, target_worker, priority DESC, created_at, id);
CREATE INDEX IF NOT EXISTS tasks_created ON tasks(created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS tasks_lease ON tasks(state, lease_expires_at);
CREATE INDEX IF NOT EXISTS tasks_deadline ON tasks(state, deadline_at);
CREATE TABLE IF NOT EXISTS workers (
    id TEXT PRIMARY KEY,
    body TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS task_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    at INTEGER NOT NULL,
    state TEXT NOT NULL,
    message TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS events_task ON task_events(task_id, sequence);
PRAGMA user_version = 1;
