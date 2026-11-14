CREATE TABLE shared_tasks (
 id TEXT PRIMARY KEY, target_worker TEXT NOT NULL, state TEXT NOT NULL,
 revision INTEGER NOT NULL, updated_at INTEGER NOT NULL, body TEXT NOT NULL
);
CREATE INDEX shared_target_recent ON shared_tasks(target_worker,updated_at DESC,id DESC);
CREATE TABLE shared_notes (
 task_id TEXT NOT NULL REFERENCES shared_tasks(id), revision INTEGER NOT NULL,
 body TEXT NOT NULL, PRIMARY KEY(task_id,revision)
);
CREATE TABLE shared_requests (
 actor TEXT NOT NULL, idempotency_key TEXT NOT NULL, request_json TEXT NOT NULL,
 reply_json TEXT NOT NULL, PRIMARY KEY(actor,idempotency_key)
);
PRAGMA user_version = 7;
CREATE TABLE app_viewers (app_id TEXT PRIMARY KEY, recipient TEXT NOT NULL, updated_at INTEGER NOT NULL);
