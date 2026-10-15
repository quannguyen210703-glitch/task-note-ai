use crate::{
    actions::{Action, Outcome},
    repository::local_stamp,
    *,
};
use chrono::{Duration, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Status,
    List {
        today: bool,
        date: Option<String>,
        status: Option<String>,
        trash: bool,
    },
    Show {
        id: String,
    },
    Add {
        title: String,
        date: Option<String>,
        time: Option<String>,
    },
    Update {
        id: String,
        patch: Patch,
        scope: Option<String>,
    },
    Complete {
        id: String,
    },
    Reopen {
        id: String,
    },
    Snooze {
        id: String,
        minutes: Option<i64>,
        until: Option<String>,
    },
    Unsnooze {
        id: String,
    },
    Delete {
        id: String,
        scope: Option<String>,
    },
    Restore {
        id: String,
    },
    Pause {
        minutes: i64,
    },
    Resume,
    BackupCreate {
        output: PathBuf,
    },
    BackupVerify {
        path: PathBuf,
    },
    Chat {
        text: String,
    },
    Plan {
        date: String,
    },
}
impl Command {
    pub fn readonly(&self) -> bool {
        matches!(
            self,
            Self::Status | Self::List { .. } | Self::Show { .. } | Self::BackupVerify { .. }
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub request_id: String,
    pub received_at_utc: i64,
    pub expires_at_utc: i64,
    pub dry_run: bool,
    pub action: Command,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fault {
    pub code: i32,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub schema_version: u32,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Fault>,
}
impl Response {
    pub fn from_result(result: Result<Value>) -> Self {
        match result {
            Ok(data) => Self {
                schema_version: 1,
                ok: true,
                data: Some(data),
                error: None,
            },
            Err(e) => Self::failure(
                match &e {
                    Error::NotFound(_) => 3,
                    Error::Ambiguous(_) => 5,
                    Error::Unavailable(_) => 7,
                    Error::Invalid(_) => 2,
                    _ => 4,
                },
                e.to_string(),
            ),
        }
    }
    pub fn failure(code: i32, message: String) -> Self {
        Self {
            schema_version: 1,
            ok: false,
            data: None,
            error: Some(Fault { code, message }),
        }
    }
}
pub fn date_value(value: &str, s: &Settings, now: i64) -> Result<NaiveDate> {
    let today = s.local_datetime(now).date_naive();
    match value {
        "today" => Ok(today),
        "tomorrow" => Ok(today + Duration::days(1)),
        _ => NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|_| Error::Invalid("Ngày cần YYYY-MM-DD, today hoặc tomorrow.".into())),
    }
}
pub fn set_date(
    input: &mut TaskInput,
    date: Option<&str>,
    time: Option<&str>,
    s: &Settings,
    now: i64,
) -> Result<()> {
    if date.is_none() && time.is_none() {
        return Ok(());
    }
    let date = match date {
        Some(d) => date_value(d, s, now)?,
        None => input
            .due_at_utc
            .map(|d| s.local_datetime(d).date_naive())
            .unwrap_or_else(|| s.local_datetime(now).date_naive()),
    };
    let clock = match time {
        Some(t) => NaiveTime::parse_from_str(t, "%H:%M")
            .map_err(|_| Error::Invalid("Giờ cần HH:MM (24h).".into()))?,
        None => NaiveTime::MIN,
    };
    input.due_at_utc = Some(local_stamp(&s.resolved_timezone()?, date.and_time(clock))?);
    input.is_all_day = time.is_none();
    Ok(())
}
impl Repository {
    pub async fn resolve_id(&self, id: &str) -> Result<Task> {
        let tasks = self.list(true).await?;
        if let Some(task) = tasks.iter().find(|t| t.id == id) {
            return Ok(task.clone());
        }
        if id.len() < 4 {
            return Err(Error::Invalid(
                "Dùng ID đầy đủ hoặc ít nhất 4 ký tự đầu.".into(),
            ));
        }
        let matches: Vec<_> = tasks.into_iter().filter(|t| t.id.starts_with(id)).collect();
        match matches.len() {
            0 => Err(Error::NotFound(
                "Không có ID này; dùng task list --json.".into(),
            )),
            1 => Ok(matches[0].clone()),
            _ => Err(Error::Ambiguous(
                "ID rút gọn trùng; chọn ID đầy đủ từ task list --json.".into(),
            )),
        }
    }
    pub async fn execute(&self, req: &Request, root: &Path, gui: bool) -> Response {
        Response::from_result(self.execute_inner(req, root, gui).await)
    }
    async fn execute_inner(&self, req: &Request, root: &Path, gui: bool) -> Result<Value> {
        let now = chrono::Utc::now().timestamp();
        if now > req.expires_at_utc || req.received_at_utc > now + 60 {
            return Err(Error::Invalid(
                "Lệnh hết hạn; thử lại với cùng request ID để kiểm tra kết quả.".into(),
            ));
        }
        let s = self.settings().await?;
        let command_json = serde_json::to_string(&req.action)?;
        if !req.dry_run && !req.action.readonly() {
            let stored:Option<(String,String,i64)>=sqlx::query_as("SELECT command_json,action_json,received_at_utc FROM service_intents WHERE request_id=?").bind(&req.request_id).fetch_optional(&self.pool).await?;
            if let Some((command, action, received)) = stored {
                if command != command_json {
                    return Err(Error::Invalid("Request ID đã dùng cho lệnh khác.".into()));
                }
                return Ok(json!(
                    self.mutate(
                        &serde_json::from_str(&action)?,
                        &req.request_id,
                        received,
                        false
                    )
                    .await?
                ));
            }
        }
        let action = match &req.action {
            Command::Status => {
                let tasks = self.list(true).await?;
                let sqlite: String = sqlx::query_scalar("SELECT sqlite_version()")
                    .fetch_one(&self.pool)
                    .await?;
                let schema: i64 = sqlx::query_scalar(
                    "SELECT coalesce(max(version),0) FROM _sqlx_migrations WHERE success=1",
                )
                .fetch_one(&self.pool)
                .await?;
                let integrity: String = sqlx::query_scalar("PRAGMA quick_check")
                    .fetch_one(&self.pool)
                    .await?;
                let fks = sqlx::query("PRAGMA foreign_key_check")
                    .fetch_all(&self.pool)
                    .await?
                    .len();
                return Ok(
                    json!({"version":env!("CARGO_PKG_VERSION"),"sqlite_version":sqlite,"schema_version":schema,"data_path":root.join("task-note.sqlite"),"timezone":s.resolved_timezone()?,"core_running":gui,"scheduler_running":gui&&scheduler_running(root),"notification_pause":s.paused&&s.pause_until_utc.is_none_or(|u|u>now),"pause_until_utc":s.pause_until_utc,"snoozed_tasks":tasks.iter().filter(|t|t.active()&&t.snoozed_until_utc.is_some_and(|u|u>now)).count(),"counts":{"todo":tasks.iter().filter(|t|t.archived_at_utc.is_none()&&t.status=="todo").count(),"doing":tasks.iter().filter(|t|t.archived_at_utc.is_none()&&t.status=="doing").count(),"done":tasks.iter().filter(|t|t.archived_at_utc.is_none()&&t.status=="done").count(),"trash":tasks.iter().filter(|t|t.archived_at_utc.is_some()).count()},"ai":{"provider":s.ai_provider,"model":s.ai_model,"enabled":s.ai_enabled,"consent":s.ai_consent,"key_configured":crate::cloud::has_key(),"auto_plan":s.auto_plan},"integrity":integrity,"foreign_key_errors":fks,"last_backup":std::fs::read_to_string(root.join("last-backup.json")).ok().and_then(|t|serde_json::from_str::<Value>(&t).ok())}),
                );
            }
            Command::List {
                today,
                date,
                status,
                trash,
            } => {
                if status
                    .as_deref()
                    .is_some_and(|v| !["todo", "doing", "done"].contains(&v))
                {
                    return Err(Error::Invalid("Trạng thái: todo, doing, done.".into()));
                }
                let date = if *today {
                    Some(date_value("today", &s, req.received_at_utc)?)
                } else {
                    date.as_deref()
                        .map(|v| date_value(v, &s, req.received_at_utc))
                        .transpose()?
                };
                return Ok(json!(self
                    .list(true)
                    .await?
                    .into_iter()
                    .filter(|t| t.archived_at_utc.is_some() == *trash
                        && status.as_ref().is_none_or(|v| v == &t.status)
                        && date.is_none_or(|d| t.local_day(&s)==Some(d)))
                    .collect::<Vec<_>>()));
            }
            Command::Show { id } => return Ok(json!(self.resolve_id(id).await?)),
            Command::Add { title, date, time } => {
                let mut input = TaskInput::new(title, None);
                set_date(
                    &mut input,
                    Some(date.as_deref().unwrap_or("today")),
                    time.as_deref(),
                    &s,
                    req.received_at_utc,
                )?;
                Action::Add {
                    inputs: vec![input],
                    source: "manual".into(),
                    original: String::new(),
                }
            }
            Command::Update { id, patch, scope } => {
                let task = self.resolve_id(id).await?;
                let mut input = task.input();
                if let Some(v) = &patch.title {
                    input.title = v.clone();
                }
                if let Some(v) = &patch.description {
                    input.description = v.clone();
                }
                if let Some(v) = &patch.status {
                    input.status = v.clone();
                }
                if let Some(v) = &patch.priority {
                    input.priority = v.clone();
                }
                set_date(
                    &mut input,
                    patch.date.as_deref(),
                    patch.time.as_deref(),
                    &s,
                    req.received_at_utc,
                )?;
                Action::Update {
                    id: task.id.clone(),
                    input,
                    expected: task.updated_at_utc,
                    scope: scope_for(&task, scope)?,
                }
            }
            Command::Complete { id } | Command::Reopen { id } => {
                let task = self.resolve_id(id).await?;
                let mut input = task.input();
                input.status = if matches!(req.action, Command::Complete { .. }) {
                    "done"
                } else {
                    "todo"
                }
                .into();
                Action::Update {
                    id: task.id,
                    input,
                    expected: task.updated_at_utc,
                    scope: "one".into(),
                }
            }
            Command::Snooze { id, minutes, until } => {
                let task = self.resolve_id(id).await?;
                let until = match (minutes, until) {
                    (Some(m), None) if (1..=525600).contains(m) => now + m * 60,
                    (None, Some(v)) => chrono::DateTime::parse_from_rfc3339(v)
                        .map_err(|_| Error::Invalid("--until cần RFC3339 kèm timezone.".into()))?
                        .timestamp(),
                    _ => {
                        return Err(Error::Invalid(
                            "Chọn --for 30m hoặc --until có múi giờ.".into(),
                        ))
                    }
                };
                Action::Snooze {
                    id: task.id,
                    until,
                    expected: task.updated_at_utc,
                }
            }
            Command::Unsnooze { id } => Action::Unsnooze {
                id: self.resolve_id(id).await?.id,
            },
            Command::Delete { id, scope } => {
                let task = self.resolve_id(id).await?;
                Action::Delete {
                    id: task.id.clone(),
                    expected: task.updated_at_utc,
                    scope: scope_for(&task, scope)?,
                }
            }
            Command::Restore { id } => Action::Restore {
                id: self.resolve_id(id).await?.id,
            },
            Command::Pause { minutes } => {
                if !(1..=525600).contains(minutes) {
                    return Err(Error::Invalid("Thời gian tạm dừng 1–525600 phút.".into()));
                }
                Action::Pause {
                    until: Some(now + minutes * 60),
                }
            }
            Command::Resume => Action::Resume,
            Command::BackupCreate { output } => {
                if req.dry_run {
                    return Ok(json!({"dry_run":true,"output":output}));
                }
                std::fs::create_dir_all(output)?;
                let path = output.join(format!(
                    "TaskNoteAI-{}-{}.sqlite",
                    chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
                    uuid::Uuid::new_v4()
                ));
                self.backup_database(&path).await?;
                let check = verify_backup(&path).await?;
                std::fs::write(
                    root.join("last-backup.json"),
                    serde_json::to_string(&json!({"path":path,"at_utc":now,"verified":true}))?,
                )?;
                return Ok(check);
            }
            Command::BackupVerify { path } => return verify_backup(path).await,
            Command::Chat { text } => {
                return Ok(json!(
                    self.chat_v2(
                        text,
                        &req.request_id,
                        req.received_at_utc,
                        None,
                        req.dry_run
                    )
                    .await?
                ))
            }
            Command::Plan { date } => {
                let date = date_value(date, &s, req.received_at_utc)?;
                return self.plan(date, req.received_at_utc, true).await;
            }
        };
        // Return a durable receipt BEFORE resolving the task again on retry. The
        // original action contains a version that may no longer be current.
        if !req.dry_run {
            sqlx::query("INSERT INTO service_intents VALUES(?,?,?,?) ON CONFLICT DO NOTHING")
                .bind(&req.request_id)
                .bind(command_json)
                .bind(serde_json::to_string(&action)?)
                .bind(req.received_at_utc)
                .execute(&self.pool)
                .await?;
        }
        let result: Outcome = self
            .mutate(&action, &req.request_id, req.received_at_utc, req.dry_run)
            .await?;
        Ok(json!(result))
    }
    pub async fn backup_database(&self, path: &Path) -> Result<()> {
        if path.exists() {
            return Err(Error::Invalid("Backup đã tồn tại; không ghi đè.".into()));
        }
        sqlx::query("VACUUM INTO ?")
            .bind(path.to_string_lossy().as_ref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
fn scope_for(task: &Task, scope: &Option<String>) -> Result<String> {
    match scope.as_deref() {
        Some("occurrence") => Ok("one".into()),
        Some("future") => Ok("series".into()),
        Some(_) => Err(Error::Invalid("--scope occurrence|future".into())),
        None if task.template_id.is_some() => Err(Error::Ambiguous(
            "Lịch lặp cần --scope occurrence hoặc future.".into(),
        )),
        None => Ok("one".into()),
    }
}
pub async fn verify_backup(path: &Path) -> Result<Value> {
    if !path.is_file() {
        return Err(Error::NotFound("Không tìm thấy file backup.".into()));
    }
    let opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .foreign_keys(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    let result:Result<Value>=async{
        let integrity:String=sqlx::query_scalar("PRAGMA integrity_check").fetch_one(&pool).await?;
        if integrity!="ok" || !sqlx::query("PRAGMA foreign_key_check").fetch_all(&pool).await?.is_empty(){return Err(Error::Invalid("Backup không đạt integrity/foreign keys.".into()));}
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM tasks").fetch_one(&pool).await?;
        let schema:i64=sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations WHERE success=1").fetch_one(&pool).await?;
        Ok(json!({"path":path,"verified":true,"integrity":integrity,"tasks":count,"schema_version":schema}))
    }.await;
    pool.close().await;
    result
}

fn scheduler_running(root:&Path)->bool {
    std::fs::read_to_string(root.join("engine-status.json")).ok()
        .and_then(|v|serde_json::from_str::<Value>(&v).ok())
        .is_some_and(|v|v["running"].as_bool()==Some(true)&&v["last_check"].as_i64().is_some_and(|t|(chrono::Utc::now().timestamp()-t).abs()<65))
}
