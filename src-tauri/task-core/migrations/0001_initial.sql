CREATE TABLE tasks (
 id TEXT PRIMARY KEY, title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 300),
 description TEXT NOT NULL DEFAULT '', due_at_utc INTEGER, is_all_day INTEGER NOT NULL DEFAULT 0,
 priority TEXT NOT NULL CHECK(priority IN ('high','medium','low')),
 status TEXT NOT NULL CHECK(status IN ('todo','doing','done')),
 tag_or_project TEXT NOT NULL DEFAULT '', recurrence_rule TEXT,
 snoozed_until_utc INTEGER, created_at_utc INTEGER NOT NULL, updated_at_utc INTEGER NOT NULL,
 completed_at_utc INTEGER, archived_at_utc INTEGER,
 template_id TEXT REFERENCES templates(id), occurrence_date TEXT,
 UNIQUE(template_id, occurrence_date)
);
CREATE TABLE templates (
 id TEXT PRIMARY KEY, input_json TEXT NOT NULL, timezone TEXT NOT NULL,
 start_date TEXT NOT NULL, generated_until TEXT, archived_at_utc INTEGER
);
CREATE INDEX tasks_due ON tasks(due_at_utc) WHERE archived_at_utc IS NULL;
CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK(id=1), json TEXT NOT NULL);
CREATE TABLE reminder_dispatches (
 dispatch_key TEXT PRIMARY KEY, kind TEXT NOT NULL, sent_at_utc INTEGER NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('submitted','failed')), error TEXT
);
CREATE TABLE proposals (token TEXT PRIMARY KEY, payload TEXT NOT NULL, created_at_utc INTEGER NOT NULL, consumed_at_utc INTEGER);
CREATE TABLE chat_history (id INTEGER PRIMARY KEY AUTOINCREMENT, role TEXT NOT NULL, content TEXT NOT NULL, created_at_utc INTEGER NOT NULL);
