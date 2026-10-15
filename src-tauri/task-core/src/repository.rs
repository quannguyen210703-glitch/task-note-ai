use crate::model::*;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Sqlite, SqlitePool, Transaction,
};
use std::path::Path;
use uuid::Uuid;

#[derive(Clone)]
pub struct Repository {
    pub pool: SqlitePool,
    pub mutation_gate: std::sync::Arc<tokio::sync::Mutex<()>>,
}
pub fn local_stamp(zone: &str, date: NaiveDateTime) -> Result<i64> {
    if zone == "system" {
        Local
            .from_local_datetime(&date)
            .single()
            .map(|d| d.timestamp())
    } else {
        zone.parse::<chrono_tz::Tz>()
            .ok()
            .and_then(|tz| tz.from_local_datetime(&date).single())
            .map(|d| d.timestamp())
    }
    .ok_or_else(|| {
        Error::Invalid("Thời gian không tồn tại hoặc không rõ ràng trong múi giờ đã chọn.".into())
    })
}
impl Repository {
    pub async fn origin(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let row:Option<String>=sqlx::query_scalar("SELECT json_object('source',source,'original_request',original_request,'goal_id',goal_id,'step_id',step_id,'request_id',request_id) FROM task_provenance WHERE task_id=?").bind(id).fetch_optional(&self.pool).await?;
        row.map(|r| serde_json::from_str(&r).map_err(Into::into))
            .transpose()
    }
    pub async fn open_readonly(path: &Path) -> Result<Self> {
        if !path.is_file() {
            return Err(Error::NotFound(
                "Chưa có DB trong profile này; mở ứng dụng hoặc thêm việc trước.".into(),
            ));
        }
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .read_only(true)
            .foreign_keys(true)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        Ok(Self {
            pool,
            mutation_gate: Default::default(),
        })
    }
    pub async fn open(path: &Path) -> Result<Self> {
        let existed = path.is_file();
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        if existed {
            let has_migrations: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM sqlite_master WHERE name='_sqlx_migrations'",
            )
            .fetch_one(&pool)
            .await?;
            if has_migrations != 1 {
                return Err(Error::Invalid(
                    "DB hiện có không đúng Task Note AI; không tự reset hoặc tạo DB thay thế."
                        .into(),
                ));
            }
            let version: i64 = sqlx::query_scalar(
                "SELECT coalesce(max(version),0) FROM _sqlx_migrations WHERE success=1",
            )
            .fetch_one(&pool)
            .await?;
            if version < 2 {
                let backup = path.with_file_name(format!("pre-v2-{}.sqlite", uuid::Uuid::new_v4()));
                sqlx::query("VACUUM INTO ?")
                    .bind(backup.to_string_lossy().as_ref())
                    .execute(&pool)
                    .await?;
                crate::service::verify_backup(&backup).await?;
            }
        }
        sqlx::migrate!("./migrations").run(&pool).await?;
        let repo = Self {
            pool,
            mutation_gate: Default::default(),
        };
        let settings = repo.settings().await?;
        let missing: Vec<Task> = sqlx::query_as(
            "SELECT * FROM tasks WHERE id NOT IN(SELECT task_id FROM task_calendar)",
        )
        .fetch_all(&repo.pool)
        .await?;
        if !missing.is_empty() {
            let mut tx = repo.pool.begin().await?;
            for task in missing {
                let date = if task.is_all_day {
                    task.due_at_utc
                        .map(|u| settings.local_datetime(u).date_naive().to_string())
                } else {
                    None
                };
                sqlx::query("INSERT INTO task_calendar VALUES(?,?,?)")
                    .bind(&task.id)
                    .bind(date)
                    .bind(settings.resolved_timezone()?)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
        }
        Ok(repo)
    }
    pub async fn memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self {
            pool,
            mutation_gate: Default::default(),
        })
    }
    pub async fn settings(&self) -> Result<Settings> {
        let json: Option<String> = sqlx::query_scalar("SELECT json FROM settings WHERE id=1")
            .fetch_optional(&self.pool)
            .await?;
        Ok(match json {
            Some(s) => serde_json::from_str(&s)?,
            None => Settings::default(),
        })
    }
    pub async fn save_settings(&self, s: &Settings) -> Result<()> {
        let _guard = self.mutation_gate.lock().await;
        s.validate()?;
        sqlx::query(
            "INSERT INTO settings VALUES(1,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json",
        )
        .bind(serde_json::to_string(s)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn list(&self, include_archived: bool) -> Result<Vec<Task>> {
        // V1 backups are readable before migration (doctor/upgrade comparison).
        let has_calendar: i64 =
            sqlx::query_scalar("SELECT count(*) FROM sqlite_master WHERE name='task_calendar'")
                .fetch_one(&self.pool)
                .await?;
        let sql = if has_calendar == 1 {
            "SELECT t.*,c.local_date,c.timezone AS task_timezone,p.source,p.goal_id,p.estimate_minutes,CASE WHEN json_extract(gs.json,'$.depends_on') IS NULL THEN 0 WHEN EXISTS(SELECT 1 FROM goal_steps prev JOIN tasks pt ON pt.id=prev.task_id WHERE prev.goal_id=p.goal_id AND prev.position=json_extract(gs.json,'$.depends_on') AND pt.status='done' AND pt.archived_at_utc IS NULL) THEN 0 ELSE 1 END AS blocked FROM tasks t LEFT JOIN task_calendar c ON c.task_id=t.id LEFT JOIN task_provenance p ON p.task_id=t.id LEFT JOIN goal_steps gs ON gs.id=p.step_id WHERE (? OR t.archived_at_utc IS NULL) ORDER BY t.due_at_utc IS NULL,t.due_at_utc,t.title"
        } else {
            "SELECT * FROM tasks WHERE (? OR archived_at_utc IS NULL) ORDER BY due_at_utc IS NULL,due_at_utc,title"
        };
        Ok(sqlx::query_as::<_, Task>(sql)
            .bind(include_archived)
            .fetch_all(&self.pool)
            .await?)
    }
    pub async fn get(&self, id: &str) -> Result<Task> {
        sqlx::query_as("SELECT t.*,c.local_date,c.timezone AS task_timezone,p.source,p.goal_id,p.estimate_minutes FROM tasks t LEFT JOIN task_calendar c ON c.task_id=t.id LEFT JOIN task_provenance p ON p.task_id=t.id WHERE t.id=?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| Error::Invalid("Không tìm thấy công việc.".into()))
    }
    pub async fn create(&self, input: &TaskInput, now: i64) -> Result<Task> {
        input.validate()?;
        let zone = self.settings().await?.resolved_timezone()?;
        let mut tx = self.pool.begin().await?;
        let task = create_in(&mut tx, input, now, &zone).await?;
        tx.commit().await?;
        if task.template_id.is_some() {
            self.materialize(now).await?;
        }
        Ok(task)
    }
    pub async fn update(
        &self,
        id: &str,
        input: &TaskInput,
        expected: i64,
        scope: &str,
        now: i64,
    ) -> Result<Task> {
        input.validate()?;
        if !["one", "series"].contains(&scope) {
            return Err(Error::Invalid("Phạm vi sửa không hợp lệ.".into()));
        }
        let mut tx = self.pool.begin().await?;
        let mut task: Task =
            sqlx::query_as("SELECT * FROM tasks WHERE id=? AND archived_at_utc IS NULL")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| Error::Invalid("Công việc đã bị xóa.".into()))?;
        if task.updated_at_utc != expected {
            return Err(Error::Invalid(
                "Công việc đã thay đổi ở cửa sổ khác. Hãy mở lại chi tiết.".into(),
            ));
        }
        if task.template_id.is_none() && input.recurrence_rule.is_some() {
            return Err(Error::Invalid(
                "Hãy tạo công việc lặp mới; không thể chuyển công việc đơn thành chuỗi khi sửa."
                    .into(),
            ));
        }
        if scope == "one" && input.recurrence_rule != task.recurrence_rule {
            return Err(Error::Invalid("Đổi quy tắc lặp cần chọn cả chuỗi.".into()));
        }
        task.title = input.title.trim().into();
        task.description = input.description.clone();
        task.due_at_utc = input.due_at_utc;
        task.is_all_day = input.is_all_day;
        task.priority = input.priority.clone();
        task.status = input.status.clone();
        task.tag_or_project = input.tag_or_project.clone();
        task.recurrence_rule = input.recurrence_rule.clone();
        task.snoozed_until_utc = None;
        task.updated_at_utc = now.max(expected + 1);
        task.completed_at_utc = if task.status == "done" {
            task.completed_at_utc.or(Some(now))
        } else {
            None
        };
        if scope == "series" {
            if let Some(template) = &task.template_id {
                let zone: String = sqlx::query_scalar("SELECT timezone FROM templates WHERE id=?")
                    .bind(template)
                    .fetch_one(&mut *tx)
                    .await?;
                let settings = Settings {
                    timezone: zone.clone(),
                    ..Settings::default()
                };
                let start = if let Some(due) = input.due_at_utc {
                    let local = settings.local_datetime(due);
                    if let Some(rule) = input.recurrence_rule.as_deref() {
                        let day = local.weekday().num_days_from_monday();
                        if (rule == "weekdays" && day >= 5)
                            || (rule.starts_with("weekly:") && rule != format!("weekly:{day}"))
                        {
                            return Err(Error::Invalid(
                                "Ngày bắt đầu chưa khớp quy tắc lặp mới.".into(),
                            ));
                        }
                    }
                    local.date_naive().to_string()
                } else {
                    task.occurrence_date
                        .clone()
                        .ok_or_else(|| Error::Invalid("Chuỗi thiếu ngày bắt đầu.".into()))?
                };
                // Legacy series API: remove derived calendar rows for exactly
                // the occurrences being replaced. History/provenance FKs remain
                // restrictive. Production UI/CLI use the preserving Action API.
                sqlx::query("DELETE FROM task_calendar WHERE task_id IN (SELECT id FROM tasks WHERE template_id=? AND id<>? AND status<>'done' AND archived_at_utc IS NULL AND occurrence_date>?)").bind(template).bind(id).bind(task.occurrence_date.as_deref().unwrap_or(&start)).execute(&mut *tx).await?;
                sqlx::query("DELETE FROM tasks WHERE template_id=? AND id<>? AND status<>'done' AND archived_at_utc IS NULL AND occurrence_date>?").bind(template).bind(id).bind(task.occurrence_date.as_deref().unwrap_or(&start)).execute(&mut *tx).await?;
                sqlx::query("UPDATE templates SET input_json=?,start_date=?,generated_until=NULL,archived_at_utc=? WHERE id=?").bind(serde_json::to_string(input)?).bind(&start).bind(if input.recurrence_rule.is_none(){Some(now)}else{None}).bind(template).execute(&mut *tx).await?;
                if input.recurrence_rule.is_some() {
                    task.occurrence_date = Some(start);
                }
            }
        }
        write_task(&mut tx, &task).await?;
        tx.commit().await?;
        if scope == "series" {
            self.materialize(now).await?;
        }
        Ok(task)
    }
    pub async fn snooze(&self, id: &str, until: i64, expected: i64, now: i64) -> Result<()> {
        if until <= now || until > 4102444800 || DateTime::from_timestamp(until, 0).is_none() {
            return Err(Error::Invalid("Giờ hoãn phải ở tương lai.".into()));
        }
        let result=sqlx::query("UPDATE tasks SET snoozed_until_utc=?,updated_at_utc=? WHERE id=? AND updated_at_utc=? AND status<>'done' AND archived_at_utc IS NULL").bind(until).bind(now.max(expected+1)).bind(id).bind(expected).execute(&self.pool).await?;
        if result.rows_affected() != 1 {
            return Err(Error::Invalid(
                "Công việc đã thay đổi hoặc đã hoàn thành.".into(),
            ));
        }
        Ok(())
    }
    pub async fn archive(&self, id: &str, scope: &str, expected: i64, now: i64) -> Result<()> {
        if !["one", "series"].contains(&scope) {
            return Err(Error::Invalid("Phạm vi xóa không hợp lệ.".into()));
        }
        let mut tx = self.pool.begin().await?;
        let task: Task = sqlx::query_as(
            "SELECT * FROM tasks WHERE id=? AND updated_at_utc=? AND archived_at_utc IS NULL",
        )
        .bind(id)
        .bind(expected)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::Invalid("Công việc đã thay đổi.".into()))?;
        if scope == "series" {
            if let Some(template) = task.template_id {
                sqlx::query("UPDATE templates SET archived_at_utc=? WHERE id=?")
                    .bind(now)
                    .bind(&template)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query(
                    "UPDATE tasks SET archived_at_utc=?,updated_at_utc=? WHERE template_id=?",
                )
                .bind(now)
                .bind(now.max(expected + 1))
                .bind(template)
                .execute(&mut *tx)
                .await?;
            }
        }
        sqlx::query("UPDATE tasks SET archived_at_utc=?,updated_at_utc=? WHERE id=?")
            .bind(now)
            .bind(now.max(expected + 1))
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn restore(&self, id: &str, now: i64) -> Result<()> {
        let task = self.get(id).await?;
        if let Some(template) = task.template_id {
            let active: Option<i64> =
                sqlx::query_scalar("SELECT archived_at_utc FROM templates WHERE id=?")
                    .bind(template)
                    .fetch_one(&self.pool)
                    .await?;
            if active.is_some() {
                return Err(Error::Invalid("Chuỗi đã dừng. Có thể xuất sao lưu hoặc tạo chuỗi mới; không khôi phục riêng kỳ của chuỗi đã dừng.".into()));
            }
        }
        sqlx::query("UPDATE tasks SET archived_at_utc=NULL,updated_at_utc=? WHERE id=?")
            .bind(now.max(task.updated_at_utc + 1))
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn materialize(&self, now: i64) -> Result<()> {
        let templates: Vec<Template> =
            sqlx::query_as("SELECT * FROM templates WHERE archived_at_utc IS NULL")
                .fetch_all(&self.pool)
                .await?;
        for template in templates {
            let input: TaskInput = serde_json::from_str(&template.input_json)?;
            let Some(rule) = &input.recurrence_rule else {
                continue;
            };
            let settings = Settings {
                timezone: template.timezone.clone(),
                ..Settings::default()
            };
            let today = settings.local_datetime(now).date_naive();
            let end = today + Duration::days(14);
            let begin = NaiveDate::parse_from_str(&template.start_date, "%Y-%m-%d")
                .map_err(|_| Error::Invalid("Ngày của chuỗi không hợp lệ.".into()))?;
            let generated = template
                .generated_until
                .as_deref()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
            let mut date = generated
                .map(|d| d + Duration::days(1))
                .unwrap_or(begin)
                .max(begin)
                .max(today - Duration::days(31));
            if date > end {
                continue;
            }
            let due = input
                .due_at_utc
                .ok_or_else(|| Error::Invalid("Lịch lặp thiếu giờ.".into()))?;
            let time = settings.local_datetime(due).time();
            let mut tx = self.pool.begin().await?;
            while date <= end {
                let weekday = date.weekday().num_days_from_monday();
                if rule == "daily"
                    || (rule == "weekdays" && weekday < 5)
                    || rule == &format!("weekly:{weekday}")
                {
                    let stamp = local_stamp(&template.timezone, date.and_time(time))?;
                    let mut task = task_from_input(&input, now);
                    task.id = Uuid::new_v4().to_string();
                    task.due_at_utc = Some(stamp);
                    task.status = "todo".into();
                    task.completed_at_utc = None;
                    task.template_id = Some(template.id.clone());
                    task.occurrence_date = Some(date.to_string());
                    task.local_date=if input.is_all_day{Some(date.to_string())}else{None};
                    task.task_timezone=Some(template.timezone.clone());
                    insert_task(&mut tx, &task).await?;
                }
                date += Duration::days(1);
            }
            sqlx::query("UPDATE templates SET generated_until=? WHERE id=?")
                .bind(end.to_string())
                .bind(&template.id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }
        Ok(())
    }
    pub async fn export(&self, now: i64) -> Result<Backup> {
        Ok(Backup {
            version: 1,
            exported_at_utc: now,
            tasks: self.list(true).await?,
            templates: sqlx::query_as("SELECT * FROM templates")
                .fetch_all(&self.pool)
                .await?,
        })
    }
    pub async fn preview_import(&self, json: &str) -> Result<ImportPreview> {
        let backup = validate_backup(json)?;
        let mut duplicate = 0;
        for task in &backup.tasks {
            let n: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM tasks WHERE id=? OR (template_id=? AND occurrence_date=?)",
            )
            .bind(&task.id)
            .bind(&task.template_id)
            .bind(&task.occurrence_date)
            .fetch_one(&self.pool)
            .await?;
            if n > 0 {
                duplicate += 1;
            }
        }
        Ok(ImportPreview {
            new_tasks: backup.tasks.len() - duplicate,
            duplicate_tasks: duplicate,
            templates: backup.templates.len(),
        })
    }
    pub async fn import(&self, json: &str) -> Result<ImportPreview> {
        let preview = self.preview_import(json).await?;
        let backup = validate_backup(json)?;
        let mut tx = self.pool.begin().await?;
        for template in &backup.templates {
            sqlx::query("INSERT INTO templates VALUES(?,?,?,?,?,?) ON CONFLICT(id) DO NOTHING")
                .bind(&template.id)
                .bind(&template.input_json)
                .bind(&template.timezone)
                .bind(&template.start_date)
                .bind(&template.generated_until)
                .bind(template.archived_at_utc)
                .execute(&mut *tx)
                .await?;
        }
        for task in &backup.tasks {
            insert_task(&mut tx, task).await?;
        }
        tx.commit().await?;
        Ok(preview)
    }
    pub async fn claim(&self, keys: &[(String, String)], now: i64) -> Result<Vec<String>> {
        let mut tx = self.pool.begin().await?;
        let mut claimed = Vec::new();
        for (key, kind) in keys {
            let result=sqlx::query("INSERT INTO reminder_dispatches(dispatch_key,kind,sent_at_utc,status) VALUES(?,?,?,'submitted') ON CONFLICT DO NOTHING").bind(key).bind(kind).bind(now).execute(&mut *tx).await?;
            if result.rows_affected() == 1 {
                claimed.push(key.clone());
            }
        }
        tx.commit().await?;
        Ok(claimed)
    }
    pub async fn dispatch_failure(&self, keys: &[String], error: &str) -> Result<()> {
        for key in keys {
            sqlx::query(
                "UPDATE reminder_dispatches SET status='failed',error=? WHERE dispatch_key=?",
            )
            .bind(error)
            .bind(key)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
    pub async fn chat_history(&self) -> Result<Vec<(String, String)>> {
        let mut rows: Vec<(String, String)> =
            sqlx::query_as("SELECT role,content FROM chat_history ORDER BY id DESC LIMIT 100")
                .fetch_all(&self.pool)
                .await?;
        rows.reverse();
        Ok(rows)
    }
    pub async fn clear_chat(&self) -> Result<()> {
        sqlx::query("DELETE FROM batch_proposals").execute(&self.pool).await?;
        sqlx::query("DELETE FROM chat_requests").execute(&self.pool).await?;
        sqlx::query("DELETE FROM chat_history")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM proposals")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn save_chat(&self, role: &str, content: &str, now: i64) -> Result<()> {
        if self.settings().await?.save_chat {
            sqlx::query("INSERT INTO chat_history(role,content,created_at_utc) VALUES(?,?,?)")
                .bind(role)
                .bind(content)
                .bind(now)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }
}
pub fn task_from_input(input: &TaskInput, now: i64) -> Task {
    Task {
        id: Uuid::new_v4().to_string(),
        title: input.title.trim().into(),
        description: input.description.clone(),
        due_at_utc: input.due_at_utc,
        is_all_day: input.is_all_day,
        priority: input.priority.clone(),
        status: input.status.clone(),
        tag_or_project: input.tag_or_project.clone(),
        recurrence_rule: input.recurrence_rule.clone(),
        snoozed_until_utc: None,
        created_at_utc: now,
        updated_at_utc: now,
        completed_at_utc: if input.status == "done" {
            Some(now)
        } else {
            None
        },
        archived_at_utc: None,
        template_id: None,
        occurrence_date: None,
        local_date: None,
        task_timezone: None,
        source: None,
        goal_id: None,
        estimate_minutes: None,
        blocked: false,
    }
}
pub async fn create_in(
    tx: &mut Transaction<'_, Sqlite>,
    input: &TaskInput,
    now: i64,
    zone: &str,
) -> Result<Task> {
    let mut task = task_from_input(input, now);
    if input.recurrence_rule.is_some() {
        let id = Uuid::new_v4().to_string();
        let settings = Settings {
            timezone: zone.into(),
            ..Settings::default()
        };
        let day = settings
            .local_datetime(input.due_at_utc.unwrap())
            .weekday()
            .num_days_from_monday();
        let rule = input.recurrence_rule.as_deref().unwrap();
        if (rule == "weekdays" && day >= 5)
            || (rule.starts_with("weekly:") && rule != format!("weekly:{day}"))
        {
            return Err(Error::Invalid(
                "Ngày bắt đầu chưa khớp lịch lặp. Chọn ngày đúng thứ đã đặt.".into(),
            ));
        }
        let date = settings
            .local_datetime(input.due_at_utc.unwrap())
            .date_naive()
            .to_string();
        sqlx::query("INSERT INTO templates(id,input_json,timezone,start_date) VALUES(?,?,?,?)")
            .bind(&id)
            .bind(serde_json::to_string(input)?)
            .bind(zone)
            .bind(&date)
            .execute(&mut **tx)
            .await?;
        task.template_id = Some(id);
        task.occurrence_date = Some(date);
    }
    insert_task(tx, &task).await?;
    let date = if input.is_all_day {
        input.due_at_utc.map(|u| {
            Settings {
                timezone: zone.into(),
                ..Settings::default()
            }
            .local_datetime(u)
            .date_naive()
            .to_string()
        })
    } else {
        None
    };
    sqlx::query("INSERT INTO task_calendar VALUES(?,?,?)")
        .bind(&task.id)
        .bind(&date)
        .bind(zone)
        .execute(&mut **tx)
        .await?;
    // Metadata is stored separately so every legacy task field stays unchanged.
    task.local_date = date;
    task.task_timezone = Some(zone.into());
    Ok(task)
}
pub async fn insert_task(tx: &mut Transaction<'_, Sqlite>, t: &Task) -> Result<()> {
    let inserted=sqlx::query("INSERT INTO tasks VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT DO NOTHING")
        .bind(&t.id)
        .bind(&t.title)
        .bind(&t.description)
        .bind(t.due_at_utc)
        .bind(t.is_all_day)
        .bind(&t.priority)
        .bind(&t.status)
        .bind(&t.tag_or_project)
        .bind(&t.recurrence_rule)
        .bind(t.snoozed_until_utc)
        .bind(t.created_at_utc)
        .bind(t.updated_at_utc)
        .bind(t.completed_at_utc)
        .bind(t.archived_at_utc)
        .bind(&t.template_id)
        .bind(&t.occurrence_date)
        .execute(&mut **tx)
        .await?;
    if inserted.rows_affected()==0{return Ok(());}
    if let Some(zone) = &t.task_timezone {
        sqlx::query("INSERT INTO task_calendar VALUES(?,?,?) ON CONFLICT(task_id) DO NOTHING")
            .bind(&t.id)
            .bind(&t.local_date)
            .bind(zone)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(source) = &t.source {
        sqlx::query("INSERT INTO task_provenance(task_id,source,request_id,goal_id,original_request,estimate_minutes) VALUES(?,?,?,?,'',?) ON CONFLICT(task_id) DO NOTHING").bind(&t.id).bind(source).bind(format!("import:{}",t.id)).bind(&t.goal_id).bind(t.estimate_minutes).execute(&mut **tx).await?;
    }
    Ok(())
}
pub async fn write_task(tx: &mut Transaction<'_, Sqlite>, t: &Task) -> Result<()> {
    sqlx::query("UPDATE tasks SET title=?,description=?,due_at_utc=?,is_all_day=?,priority=?,status=?,tag_or_project=?,recurrence_rule=?,snoozed_until_utc=?,updated_at_utc=?,completed_at_utc=?,archived_at_utc=?,occurrence_date=? WHERE id=?")
    .bind(&t.title).bind(&t.description).bind(t.due_at_utc).bind(t.is_all_day).bind(&t.priority).bind(&t.status).bind(&t.tag_or_project).bind(&t.recurrence_rule).bind(t.snoozed_until_utc).bind(t.updated_at_utc).bind(t.completed_at_utc).bind(t.archived_at_utc).bind(&t.occurrence_date).bind(&t.id).execute(&mut **tx).await?;
    Ok(())
}
pub fn validate_backup(json: &str) -> Result<Backup> {
    if json.len() > 20_000_000 {
        return Err(Error::Invalid("Backup vượt quá 20 MB.".into()));
    }
    let backup: Backup = serde_json::from_str(json)?;
    if backup.version != 1 || backup.tasks.len() > 100_000 || backup.templates.len() > 10_000 {
        return Err(Error::Invalid(
            "Phiên bản hoặc số lượng bản sao lưu không hợp lệ.".into(),
        ));
    }
    let mut ids = std::collections::HashSet::new();
    let mut occurrences = std::collections::HashSet::new();
    let mut template_ids = std::collections::HashSet::new();
    for t in &backup.templates {
        if Uuid::parse_str(&t.id).is_err() || !template_ids.insert(t.id.clone()) {
            return Err(Error::Invalid("ID chuỗi lặp không hợp lệ/trùng.".into()));
        }
        let input: TaskInput = serde_json::from_str(&t.input_json)?;
        input.validate()?;
        Settings {
            timezone: t.timezone.clone(),
            ..Settings::default()
        }
        .validate()?;
        if NaiveDate::parse_from_str(&t.start_date, "%Y-%m-%d").is_err()
            || t.generated_until
                .as_deref()
                .is_some_and(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err())
        {
            return Err(Error::Invalid("Ngày chuỗi không hợp lệ.".into()));
        }
    }
    for t in &backup.tasks {
        t.input().validate()?;
        for value in [
            Some(t.created_at_utc),
            Some(t.updated_at_utc),
            t.completed_at_utc,
            t.archived_at_utc,
            t.snoozed_until_utc,
        ]
        .into_iter()
        .flatten()
        {
            if !(0..=4102444800).contains(&value) {
                return Err(Error::Invalid(
                    "Timestamp trong backup không hợp lệ.".into(),
                ));
            }
        }
        if t.updated_at_utc < t.created_at_utc {
            return Err(Error::Invalid(
                "Phiên bản task trong backup không hợp lệ.".into(),
            ));
        }
        if Uuid::parse_str(&t.id).is_err() || !ids.insert(t.id.clone()) {
            return Err(Error::Invalid("ID công việc không hợp lệ/trùng.".into()));
        }
        if let Some(id) = &t.template_id {
            if !template_ids.contains(id)
                || t.occurrence_date
                    .as_deref()
                    .is_none_or(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err())
                || !occurrences.insert((id.clone(), t.occurrence_date.clone()))
            {
                return Err(Error::Invalid(
                    "Occurrence không hợp lệ/trùng hoặc thiếu template.".into(),
                ));
            }
        } else if t.occurrence_date.is_some() {
            return Err(Error::Invalid("Occurrence thiếu template.".into()));
        }
    }
    Ok(backup)
}
