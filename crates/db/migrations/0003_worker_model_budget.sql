ALTER TABLE operations ADD COLUMN kind TEXT NOT NULL DEFAULT 'diagnostic'
    CHECK (kind IN ('diagnostic', 'assisted_diagnostic'));
ALTER TABLE operations ADD COLUMN assisted_json TEXT;
CREATE TABLE model_calls (
    request_id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL REFERENCES operations(operation_id),
    task_id TEXT NOT NULL,
    reservation_json TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('reserved','settled','uncertain','overrun')),
    reserved_tokens INTEGER NOT NULL CHECK (reserved_tokens > 0),
    charged_tokens INTEGER NOT NULL CHECK (charged_tokens >= 0),
    usage_json TEXT,
    created_at INTEGER NOT NULL CHECK (created_at >= 0),
    updated_at INTEGER NOT NULL CHECK (updated_at >= 0)
);
CREATE INDEX model_calls_operation ON model_calls(operation_id, state);
CREATE INDEX model_calls_period ON model_calls(created_at, charged_tokens);
CREATE INDEX model_calls_updated ON model_calls(updated_at DESC);
PRAGMA user_version = 3;
