CREATE TABLE feed_items (
 id TEXT PRIMARY KEY, dedupe_key TEXT NOT NULL UNIQUE, created_at INTEGER NOT NULL,
 kind TEXT NOT NULL, role TEXT NOT NULL, text TEXT NOT NULL, search_text TEXT NOT NULL,
 task_id TEXT, artifact_id TEXT, body TEXT NOT NULL
);
CREATE INDEX feed_day ON feed_items(created_at DESC,id DESC);
CREATE INDEX feed_kind_day ON feed_items(kind,created_at DESC,id DESC);
CREATE TABLE chat_requests (idempotency_key TEXT PRIMARY KEY,request_json TEXT NOT NULL,reply_json TEXT NOT NULL);
PRAGMA user_version = 5;
CREATE UNIQUE INDEX tasks_operation_id ON tasks(json_extract(body,'$.operation_id'));
CREATE TABLE service_updates (operation_id TEXT PRIMARY KEY,state TEXT NOT NULL,updated_at INTEGER NOT NULL);
