CREATE TABLE computer_steps (
 id TEXT PRIMARY KEY, parent_id TEXT NOT NULL REFERENCES tasks(id),
 sequence INTEGER NOT NULL CHECK(sequence BETWEEN 1 AND 8), body TEXT NOT NULL,
 UNIQUE(parent_id,sequence)
);
CREATE INDEX computer_step_parent ON computer_steps(parent_id,sequence);
PRAGMA user_version = 9;
