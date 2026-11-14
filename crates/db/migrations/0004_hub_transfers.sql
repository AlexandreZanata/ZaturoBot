CREATE TABLE file_transfers (
    id TEXT PRIMARY KEY,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    request_json TEXT NOT NULL,
    state TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    uploaded_bytes INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    body TEXT NOT NULL,
    UNIQUE(sender, idempotency_key)
);
CREATE INDEX transfer_inbox ON file_transfers(recipient, state, created_at, id);
CREATE INDEX transfer_sender ON file_transfers(sender, created_at, id);
CREATE INDEX transfer_expiry ON file_transfers(state, expires_at, id);
CREATE TABLE transfer_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    transfer_id TEXT NOT NULL REFERENCES file_transfers(id),
    at INTEGER NOT NULL,
    state TEXT NOT NULL,
    message TEXT NOT NULL
);
PRAGMA user_version = 4;
