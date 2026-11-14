CREATE TABLE assistant_calls (
 request_id TEXT PRIMARY KEY,
 operation_id TEXT NOT NULL REFERENCES service_operations(operation_id),
 task_id TEXT NOT NULL, reservation_json TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('reserved','settled','uncertain','overrun')),
 charged_tokens INTEGER NOT NULL CHECK(charged_tokens>=0), usage_json TEXT,
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE INDEX assistant_calls_period ON assistant_calls(created_at,charged_tokens);
PRAGMA user_version = 8;
