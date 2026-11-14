CREATE TABLE service_operations (
 operation_id TEXT PRIMARY KEY, task_id TEXT NOT NULL, request_json TEXT NOT NULL,
 result_json TEXT, started_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
 last_notified_state TEXT, last_polled_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX service_operations_pending ON service_operations(last_polled_at,operation_id);
PRAGMA user_version = 6;
