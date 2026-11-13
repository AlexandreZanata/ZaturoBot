CREATE TABLE IF NOT EXISTS operations (
    operation_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    result_json TEXT
);
PRAGMA user_version = 2;
