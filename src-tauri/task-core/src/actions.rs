//! The mutation boundary shared by GUI, CLI and assistant. Receipt + event + writes
//! commit together, so a retried request never repeats a successful mutation.
use crate::{
    model::*,
    repository::{create_in, write_task},
    Repository,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    ConfigureGoal {
        goal: crate::goals::Goal,
        steps: Vec<crate::cloud::AiTask>,
    },
    GoalBatch {
        goal: crate::goals::Goal,
        steps: Vec<crate::cloud::AiTask>,
        original: String,
    },
    Add {
        inputs: Vec<TaskInput>,
        source: String,
        original: String,
    },
    Update {
        id: String,
        input: TaskInput,
        expected: i64,
        scope: String,
    },
    Snooze {
        id: String,
        until: i64,
        expected: i64,
    },
    Unsnooze {
        id: String,
    },
    Delete {
        id: String,
        expected: i64,
        scope: String,
    },
    Restore {
        id: String,
    },
    Pause {
        until: Option<i64>,
    },
    Resume,
    Undo {
        request_id: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub message: String,
    pub tasks: Vec<Task>,
}
impl Repository {
    pub async fn mutate(
        &self,
        action: &Action,
        request_id: &str,
        now: i64,
        dry_run: bool,
    ) -> Result<Outcome> {
        let _guard = self.mutation_gate.lock().await;
        if request_id.is_empty()
            || request_id.len() > 120
            || request_id.chars().any(char::is_control)
        {
            return Err(Error::Invalid("Request ID cần 1–120 ký tự.".into()));
        }
        let action_json = serde_json::to_string(action)?;
        let settings = self.settings().await?;
        if dry_run {
            return self.preview_action(action, now, &settings).await;
        }
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if let Some((previous, result)) = sqlx::query_as::<_, (String, String)>(
            "SELECT action_json,result_json FROM mutation_requests WHERE request_id=?",
        )
        .bind(request_id)
        .fetch_optional(&mut *tx)
        .await?
        {
            if previous != action_json {
                return Err(Error::Invalid(
                    "Request ID đã dùng cho nội dung khác.".into(),
                ));
            }
            return Ok(serde_json::from_str(&result)?);
        }
        let mut output = Outcome {
            message: "Đã lưu.".into(),
            tasks: vec![],
        };
        match action {
            Action::ConfigureGoal { goal, steps } => {
                goal.validate()?;
                crate::cloud::validate_reply(
                    &crate::cloud::AiReply {
                        action: "propose_goal_steps".into(),
                        message: String::new(),
                        tasks: steps.clone(),
                    },
                    &settings,
                    now,
                )?;
                sqlx::query("INSERT INTO goals VALUES(?,?,?)")
                    .bind(&goal.id)
                    .bind(serde_json::to_string(goal)?)
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;
                for (i, step) in steps.iter().enumerate() {
                    sqlx::query("INSERT INTO goal_steps(id,goal_id,position,json) VALUES(?,?,?,?)")
                        .bind(uuid::Uuid::new_v4().to_string())
                        .bind(&goal.id)
                        .bind(i as i64)
                        .bind(serde_json::to_string(step)?)
                        .execute(&mut *tx)
                        .await?;
                }
                let mut settings = settings.clone();
                settings.auto_plan = true;
                sqlx::query("INSERT INTO settings VALUES(1,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json").bind(serde_json::to_string(&settings)?).execute(&mut *tx).await?;
                output.message=format!("Đã bật AI tự lập việc theo mục tiêu ‘{}’, ngày cho phép {:?}, tối đa {} việc/ngày toàn cục. Mở Mục tiêu để chỉnh sửa/tạm dừng hoặc Cài đặt để tắt.",goal.title,goal.allowed_days,settings.ai_daily_task_limit);
            }
            Action::GoalBatch {
                goal,
                steps,
                original,
            } => {
                goal.validate()?;
                if !(3..=5).contains(&steps.len()) {
                    return Err(Error::Invalid("Kế hoạch cần 3–5 bước.".into()));
                }
                let inputs = crate::cloud::validate_reply(
                    &crate::cloud::AiReply {
                        action: "propose_goal_steps".into(),
                        message: String::new(),
                        tasks: steps.clone(),
                    },
                    &settings,
                    now,
                )?;
                sqlx::query("INSERT INTO goals VALUES(?,?,?)")
                    .bind(&goal.id)
                    .bind(serde_json::to_string(goal)?)
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;
                for (i, (input, step)) in inputs.iter().zip(steps).enumerate() {
                    let mut task =
                        create_in(&mut tx, input, now, &settings.resolved_timezone()?).await?;
                    let sid = uuid::Uuid::new_v4().to_string();
                    sqlx::query("INSERT INTO goal_steps(id,goal_id,position,json,task_id) VALUES(?,?,?,?,?)").bind(&sid).bind(&goal.id).bind(i as i64).bind(serde_json::to_string(step)?).bind(&task.id).execute(&mut *tx).await?;
                    sqlx::query("INSERT INTO task_provenance(task_id,source,request_id,goal_id,step_id,original_request,estimate_minutes) VALUES(?,'goal',?,?,?,?,?)")
                        .bind(&task.id).bind(request_id).bind(&goal.id).bind(&sid).bind(original).bind(step.estimate_minutes).execute(&mut *tx).await?;
                    task.source = Some("goal".into());
                    task.goal_id = Some(goal.id.clone());
                    task.estimate_minutes = step.estimate_minutes.map(i64::from);
                    record(&mut tx, None, &task, "create", now).await?;
                    output.tasks.push(task);
                }
                output.message = format!("Đã thêm {} bước trong mục tiêu.", output.tasks.len());
            }
            Action::Add {
                inputs,
                source,
                original,
            } => {
                if inputs.is_empty()
                    || inputs.len() > 50
                    || original.len() > 16000
                    || !["manual", "chat", "goal", "planner"].contains(&source.as_str())
                {
                    return Err(Error::Invalid("Batch không hợp lệ (1–50 việc).".into()));
                }
                for input in inputs {
                    input.validate()?;
                }
                for input in inputs {
                    let task =
                        create_in(&mut tx, input, now, &settings.resolved_timezone()?).await?;
                    sqlx::query("INSERT INTO task_provenance(task_id,source,request_id,original_request) VALUES(?,?,?,?)")
                        .bind(&task.id).bind(source).bind(request_id).bind(original).execute(&mut *tx).await?;
                    record(&mut tx, None, &task, "create", now).await?;
                    output.tasks.push(task);
                }
                output.message = format!("Đã thêm {} việc.", output.tasks.len());
            }
            Action::Pause { .. } | Action::Resume => {
                let mut s = settings;
                s.paused = matches!(action, Action::Pause { .. });
                s.pause_until_utc = if let Action::Pause { until } = action {
                    *until
                } else {
                    None
                };
                if s.pause_until_utc.is_some_and(|u| u <= now) {
                    return Err(Error::Invalid(
                        "Thời gian tạm dừng phải ở tương lai.".into(),
                    ));
                }
                s.validate()?;
                sqlx::query("INSERT INTO settings VALUES(1,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json")
                    .bind(serde_json::to_string(&s)?).execute(&mut *tx).await?;
                output.message = if s.paused {
                    "Thông báo đang tạm dừng"
                } else {
                    "Đã bật lại thông báo; giữ hoãn riêng từng việc"
                }
                .into();
            }
            Action::Undo { request_id: batch } => {
                let tasks: Vec<Task> = sqlx::query_as("SELECT t.* FROM tasks t JOIN task_provenance p ON p.task_id=t.id WHERE p.request_id=?")
                    .bind(batch).fetch_all(&mut *tx).await?;
                if tasks.is_empty() {
                    return Err(Error::Invalid("Không tìm thấy batch để hoàn tác.".into()));
                }
                for mut task in tasks {
                    let events: i64 =
                        sqlx::query_scalar("SELECT count(*) FROM task_events WHERE task_id=?")
                            .bind(&task.id)
                            .fetch_one(&mut *tx)
                            .await?;
                    if events != 1
                        || task.status != "todo"
                        || task.archived_at_utc.is_some()
                        || task.snoozed_until_utc.is_some()
                    {
                        return Err(Error::Invalid(
                            "Batch đã có chỉnh sửa sau khi tạo; hãy chọn việc cụ thể để xóa."
                                .into(),
                        ));
                    }
                    let before = task.clone();
                    task.archived_at_utc = Some(now);
                    task.updated_at_utc = now.max(task.updated_at_utc + 1);
                    write_task(&mut tx, &task).await?;
                    record(&mut tx, Some(&before), &task, "undo", now).await?;
                    output.tasks.push(task);
                }
                output.message = "Đã hoàn tác batch vào Thùng rác.".into();
            }
            _ => {
                let id = match action {
                    Action::Update { id, .. }
                    | Action::Snooze { id, .. }
                    | Action::Unsnooze { id }
                    | Action::Delete { id, .. }
                    | Action::Restore { id } => id,
                    _ => unreachable!(),
                };
                let mut task: Task = sqlx::query_as("SELECT * FROM tasks WHERE id=?")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or_else(|| {
                        Error::NotFound(
                            "Không tìm thấy công việc; dùng task list để lấy ID.".into(),
                        )
                    })?;
                let before = task.clone();
                let expected = match action {
                    Action::Update { expected, .. }
                    | Action::Snooze { expected, .. }
                    | Action::Delete { expected, .. } => Some(*expected),
                    _ => None,
                };
                if expected.is_some_and(|e| e != task.updated_at_utc) {
                    return Err(Error::Invalid(
                        "Công việc đã thay đổi ở cửa sổ khác. Hãy mở lại chi tiết.".into(),
                    ));
                }
                if !matches!(action, Action::Restore { .. } | Action::Unsnooze { .. })
                    && task.archived_at_utc.is_some()
                {
                    return Err(Error::Invalid("Công việc đã ở Thùng rác.".into()));
                }
                let name = match action {
                    Action::Update { input, scope, .. } => {
                        input.validate()?;
                        valid_scope(scope)?;
                        if input.recurrence_rule != task.recurrence_rule
                            && (scope != "series" || task.template_id.is_none())
                        {
                            return Err(Error::Invalid(
                                "Đổi lịch lặp cần chọn phạm vi các lần sau.".into(),
                            ));
                        }
                        let schedule_changed = task.due_at_utc != input.due_at_utc
                            || task.is_all_day != input.is_all_day;
                        task.title = input.title.trim().into();
                        task.description = input.description.clone();
                        task.due_at_utc = input.due_at_utc;
                        task.is_all_day = input.is_all_day;
                        task.priority = input.priority.clone();
                        task.status = input.status.clone();
                        task.tag_or_project = input.tag_or_project.clone();
                        task.recurrence_rule = input.recurrence_rule.clone();
                        if task.status == "done" {
                            task.completed_at_utc = task.completed_at_utc.or(Some(now));
                            task.snoozed_until_utc = None;
                        } else {
                            task.completed_at_utc = None;
                            if schedule_changed {
                                task.snoozed_until_utc = None;
                            }
                        }
                        if scope == "series" && task.template_id.is_some() {
                            // Editing a series is deliberately explicit. Preserve every existing
                            // occurrence ID/history rather than delete/recreate them.
                            let tid = task.template_id.as_ref().unwrap();
                            let zone: String =
                                sqlx::query_scalar("SELECT timezone FROM templates WHERE id=?")
                                    .bind(tid)
                                    .fetch_one(&mut *tx)
                                    .await?;
                            let series_zone = Settings {
                                timezone: zone.clone(),
                                ..Settings::default()
                            };
                            if input.recurrence_rule != before.recurrence_rule
                                || input.due_at_utc.is_none()
                                || before.due_at_utc.is_none()
                                || (schedule_changed
                                    && series_zone
                                        .local_datetime(input.due_at_utc.unwrap())
                                        .date_naive()
                                        != series_zone
                                            .local_datetime(before.due_at_utc.unwrap())
                                            .date_naive())
                            {
                                return Err(Error::Invalid("Đổi ngày bắt đầu/quy tắc cả chuỗi cần tạo lịch mới để giữ lịch sử; chọn Lần này để đổi ngày riêng. Có thể đổi giờ cả chuỗi trong cùng ngày bắt đầu.".into()));
                            }
                            let later: Vec<Task> = sqlx::query_as("SELECT * FROM tasks WHERE template_id=? AND occurrence_date>? AND status<>'done' AND archived_at_utc IS NULL")
                                .bind(tid).bind(&task.occurrence_date).fetch_all(&mut *tx).await?;
                            for mut other in later {
                                let old = other.clone();
                                other.title = task.title.clone();
                                other.description = task.description.clone();
                                other.priority = task.priority.clone();
                                other.tag_or_project = task.tag_or_project.clone();
                                if schedule_changed {
                                    let date = other
                                        .occurrence_date
                                        .as_deref()
                                        .and_then(|d| {
                                            chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()
                                        })
                                        .ok_or_else(|| {
                                            Error::Invalid("Occurrence thiếu ngày.".into())
                                        })?;
                                    let time = series_zone
                                        .local_datetime(input.due_at_utc.unwrap())
                                        .time();
                                    other.due_at_utc = Some(crate::repository::local_stamp(
                                        &zone,
                                        date.and_time(time),
                                    )?);
                                    other.is_all_day = input.is_all_day;
                                    other.snoozed_until_utc = None;
                                }
                                other.updated_at_utc = now.max(other.updated_at_utc + 1);
                                write_task(&mut tx, &other).await?;
                                record(&mut tx, Some(&old), &other, "update", now).await?;
                            }
                            let mut template_input = input.clone();
                            let original_json: String =
                                sqlx::query_scalar("SELECT input_json FROM templates WHERE id=?")
                                    .bind(tid)
                                    .fetch_one(&mut *tx)
                                    .await?;
                            let original: TaskInput = serde_json::from_str(&original_json)?;
                            template_input.due_at_utc = original
                                .due_at_utc
                                .map(|d| {
                                    if schedule_changed {
                                        crate::repository::local_stamp(
                                            &zone,
                                            series_zone.local_datetime(d).date_naive().and_time(
                                                series_zone
                                                    .local_datetime(input.due_at_utc.unwrap())
                                                    .time(),
                                            ),
                                        )
                                    } else {
                                        Ok(d)
                                    }
                                })
                                .transpose()?;
                            sqlx::query("UPDATE templates SET input_json=? WHERE id=?")
                                .bind(serde_json::to_string(&template_input)?)
                                .bind(tid)
                                .execute(&mut *tx)
                                .await?;
                            let _ = zone;
                        }
                        "update"
                    }
                    Action::Snooze { until, .. } => {
                        if !task.active() || *until <= now || *until > 4102444800 {
                            return Err(Error::Invalid(
                                "Chỉ hoãn việc chưa xong tới thời điểm tương lai (trước 2100)."
                                    .into(),
                            ));
                        }
                        task.snoozed_until_utc = Some(*until);
                        "snooze"
                    }
                    Action::Unsnooze { .. } => {
                        if task.snoozed_until_utc.is_none() {
                            output.message = "Việc này không còn tạm hoãn".into();
                            output.tasks.push(task);
                            if !dry_run {
                                save_receipt(&mut tx, request_id, &action_json, &output, now)
                                    .await?;
                                tx.commit().await?;
                            }
                            return Ok(output);
                        }
                        task.snoozed_until_utc = None;
                        output.message = if task.active() {
                            "Đã bỏ tạm hoãn"
                        } else {
                            "Đã bỏ thời điểm hoãn; việc đã xong/xóa vẫn không nhắc"
                        }
                        .into();
                        "unsnooze"
                    }
                    Action::Delete { scope, .. } => {
                        valid_scope(scope)?;
                        if scope == "series" {
                            if let Some(tid) = &task.template_id {
                                sqlx::query("UPDATE templates SET archived_at_utc=? WHERE id=?")
                                    .bind(now)
                                    .bind(tid)
                                    .execute(&mut *tx)
                                    .await?;
                                let later: Vec<Task> = sqlx::query_as("SELECT * FROM tasks WHERE template_id=? AND occurrence_date>? AND status<>'done' AND archived_at_utc IS NULL")
                                    .bind(tid).bind(&task.occurrence_date).fetch_all(&mut *tx).await?;
                                for mut other in later {
                                    let old = other.clone();
                                    other.archived_at_utc = Some(now);
                                    other.updated_at_utc = now.max(other.updated_at_utc + 1);
                                    write_task(&mut tx, &other).await?;
                                    record(&mut tx, Some(&old), &other, "delete", now).await?;
                                }
                            }
                        }
                        task.archived_at_utc = Some(now);
                        output.message = "Đã xóa việc vào Thùng rác. Có thể khôi phục.".into();
                        "delete"
                    }
                    Action::Restore { .. } => {
                        task.archived_at_utc = None;
                        if task.snoozed_until_utc.is_some_and(|u| u <= now) {
                            task.snoozed_until_utc = None;
                        }
                        output.message =
                            "Đã khôi phục đúng ID; chuỗi đã dừng không tự bật lại.".into();
                        "restore"
                    }
                    _ => unreachable!(),
                };
                task.updated_at_utc = now.max(task.updated_at_utc + 1);
                write_task(&mut tx, &task).await?;
                if matches!(action, Action::Update { .. })
                    && (task.due_at_utc != before.due_at_utc
                        || task.is_all_day != before.is_all_day)
                {
                    let date = if task.is_all_day {
                        task.due_at_utc
                            .map(|u| settings.local_datetime(u).date_naive().to_string())
                    } else {
                        None
                    };
                    sqlx::query("INSERT INTO task_calendar VALUES(?,?,?) ON CONFLICT(task_id) DO UPDATE SET local_date=excluded.local_date,timezone=excluded.timezone")
                        .bind(&task.id).bind(&date).bind(settings.resolved_timezone()?).execute(&mut *tx).await?;
                    task.local_date = date;
                    task.task_timezone = Some(settings.resolved_timezone()?);
                }
                record(&mut tx, Some(&before), &task, name, now).await?;
                output.tasks.push(task);
            }
        }
        if dry_run {
            tx.rollback().await?;
            output.message = format!("Dự kiến (chưa lưu): {}", output.message);
        } else {
            save_receipt(&mut tx, request_id, &action_json, &output, now).await?;
            tx.commit().await?;
        }
        Ok(output)
    }
    async fn preview_action(&self, action: &Action, now: i64, s: &Settings) -> Result<Outcome> {
        let mut tasks = vec![];
        match action {
            Action::ConfigureGoal { goal, .. } => {
                goal.validate()?;
            }
            Action::GoalBatch { goal, steps, .. } => {
                goal.validate()?;
                let inputs = crate::cloud::validate_reply(
                    &crate::cloud::AiReply {
                        action: "propose_goal_steps".into(),
                        message: String::new(),
                        tasks: steps.clone(),
                    },
                    s,
                    now,
                )?;
                tasks = inputs
                    .iter()
                    .map(|i| crate::repository::task_from_input(i, now))
                    .collect();
            }
            Action::Add { inputs, .. } => {
                if inputs.is_empty() || inputs.len() > 50 {
                    return Err(Error::Invalid("Batch cần 1–50 việc.".into()));
                }
                for input in inputs {
                    input.validate()?;
                    tasks.push(crate::repository::task_from_input(input, now));
                }
            }
            Action::Update {
                id,
                input,
                expected,
                scope,
            } => {
                input.validate()?;
                valid_scope(scope)?;
                let mut t = self.get(id).await?;
                if t.updated_at_utc != *expected || t.archived_at_utc.is_some() {
                    return Err(Error::Invalid("Công việc đã thay đổi/xóa.".into()));
                }
                t.title = input.title.clone();
                t.description = input.description.clone();
                t.due_at_utc = input.due_at_utc;
                t.is_all_day = input.is_all_day;
                t.status = input.status.clone();
                t.priority = input.priority.clone();
                tasks.push(t);
            }
            Action::Snooze { id, until, .. } => {
                let mut t = self.get(id).await?;
                if !t.active() || *until <= now {
                    return Err(Error::Invalid(
                        "Không thể hoãn việc xong/xóa hoặc tới giờ đã qua.".into(),
                    ));
                }
                t.snoozed_until_utc = Some(*until);
                tasks.push(t);
            }
            Action::Unsnooze { id } => {
                let mut t = self.get(id).await?;
                t.snoozed_until_utc = None;
                tasks.push(t);
            }
            Action::Delete { id, scope, .. } => {
                valid_scope(scope)?;
                let mut t = self.get(id).await?;
                t.archived_at_utc = Some(now);
                tasks.push(t);
            }
            Action::Restore { id } => {
                let mut t = self.get(id).await?;
                t.archived_at_utc = None;
                tasks.push(t);
            }
            Action::Pause { until } => {
                if until.is_some_and(|u| u <= now) {
                    return Err(Error::Invalid("Thời gian phải ở tương lai.".into()));
                }
            }
            Action::Undo { .. } | Action::Resume => {}
        }
        let _ = s;
        Ok(Outcome {
            message: "Dự kiến; chưa lưu hoặc thay đổi lịch nhắc.".into(),
            tasks,
        })
    }
}
fn valid_scope(scope: &str) -> Result<()> {
    if !["one", "series"].contains(&scope) {
        return Err(Error::Invalid(
            "Chọn occurrence hoặc future cho lịch lặp.".into(),
        ));
    }
    Ok(())
}
pub(crate) async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    before: Option<&Task>,
    after: &Task,
    action: &str,
    now: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO task_events(task_id,action,before_json,after_json,at_utc) VALUES(?,?,?,?,?)",
    )
    .bind(&after.id)
    .bind(action)
    .bind(before.map(serde_json::to_string).transpose()?)
    .bind(serde_json::to_string(after)?)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
async fn save_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    action: &str,
    result: &Outcome,
    now: i64,
) -> Result<()> {
    sqlx::query("INSERT INTO mutation_requests VALUES(?,?,?,?)")
        .bind(id)
        .bind(action)
        .bind(serde_json::to_string(result)?)
        .bind(now)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
