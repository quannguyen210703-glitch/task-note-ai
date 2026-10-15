-- Additive only: all V1 columns, IDs, settings, reminder history remain intact.
CREATE TABLE mutation_requests (
 request_id TEXT PRIMARY KEY, action_json TEXT NOT NULL, result_json TEXT NOT NULL,
 received_at_utc INTEGER NOT NULL
);
CREATE TABLE task_events (
 id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL REFERENCES tasks(id),
 action TEXT NOT NULL, before_json TEXT, after_json TEXT NOT NULL, at_utc INTEGER NOT NULL
);
CREATE TABLE task_provenance (
 task_id TEXT PRIMARY KEY REFERENCES tasks(id), source TEXT NOT NULL,
 request_id TEXT NOT NULL, goal_id TEXT, step_id TEXT, original_request TEXT NOT NULL,
 planned_at_utc INTEGER, estimate_minutes INTEGER, period TEXT
);
CREATE TABLE goals (
 id TEXT PRIMARY KEY, json TEXT NOT NULL, updated_at_utc INTEGER NOT NULL
);
CREATE TABLE goal_steps (
 id TEXT PRIMARY KEY, goal_id TEXT NOT NULL REFERENCES goals(id),
 position INTEGER NOT NULL, json TEXT NOT NULL, task_id TEXT REFERENCES tasks(id),
 dismissed INTEGER NOT NULL DEFAULT 0, UNIQUE(goal_id,position)
);
CREATE TABLE plan_runs (
 local_date TEXT PRIMARY KEY, status TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0,
 last_attempt_utc INTEGER NOT NULL, error TEXT
);
CREATE TABLE chat_requests (
 request_id TEXT PRIMARY KEY, original_text TEXT NOT NULL, received_at_utc INTEGER NOT NULL,
 context_json TEXT NOT NULL, reply_json TEXT, pending_json TEXT
);
CREATE TABLE batch_proposals (
 token TEXT PRIMARY KEY, payload_json TEXT NOT NULL, created_at_utc INTEGER NOT NULL,
 consumed_at_utc INTEGER
);
CREATE TABLE ui_drafts (key TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE service_intents (request_id TEXT PRIMARY KEY, command_json TEXT NOT NULL, action_json TEXT NOT NULL, received_at_utc INTEGER NOT NULL);
CREATE TABLE task_calendar (task_id TEXT PRIMARY KEY REFERENCES tasks(id), local_date TEXT, timezone TEXT NOT NULL);
CREATE TABLE ai_calls (id TEXT PRIMARY KEY,local_date TEXT NOT NULL,goal_id TEXT NOT NULL,at_utc INTEGER NOT NULL,status TEXT NOT NULL);
