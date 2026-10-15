use crate::{
    cloud::{self, AiTask},
    repository::create_in,
    *,
};
use chrono::{Datelike, NaiveDate, Timelike};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub desired_outcome: String,
    pub start_date: String,
    pub deadline: Option<String>,
    pub allowed_days: Vec<u32>,
    pub auto_add: bool,
    pub status: String,
    pub missing_information: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub goal_id: String,
    pub position: i64,
    pub input: AiTask,
    pub task_id: Option<String>,
    pub dismissed: bool,
}
impl Goal {
    pub(crate) fn validate(&self) -> Result<()> {
        if uuid::Uuid::parse_str(&self.id).is_err()
            || self.title.trim().is_empty()
            || self.title.chars().count() > 300
            || self.description.chars().count() > 10000
            || self.desired_outcome.chars().count() > 10000
            || self.missing_information.len() > 10000
            || !["active", "paused", "done"].contains(&self.status.as_str())
            || self.allowed_days.iter().any(|d| *d > 6)
        {
            return Err(Error::Invalid("Mục tiêu không hợp lệ.".into()));
        }
        NaiveDate::parse_from_str(&self.start_date, "%Y-%m-%d")
            .map_err(|_| Error::Invalid("Ngày bắt đầu cần YYYY-MM-DD.".into()))?;
        if let Some(d) = &self.deadline {
            NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map_err(|_| Error::Invalid("Hạn mục tiêu cần YYYY-MM-DD.".into()))?;
            if d < &self.start_date {
                return Err(Error::Invalid("Hạn mục tiêu trước ngày bắt đầu.".into()));
            }
        }
        Ok(())
    }
}
impl Repository {
    pub async fn set_step_estimate(
        &self,
        step_id: &str,
        minutes: Option<i64>,
        now: i64,
    ) -> Result<()> {
        if minutes.is_some_and(|m| !(1..=600).contains(&m)) {
            return Err(Error::Invalid("Thời lượng cần 1–600 phút.".into()));
        }
        let _guard = self.mutation_gate.lock().await;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT task_id FROM goal_steps WHERE id=?")
                .bind(step_id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some((task_id,)) = row else {
            return Err(Error::NotFound("Không tìm thấy bước mục tiêu.".into()));
        };
        sqlx::query("UPDATE goal_steps SET json=json_set(json,'$.estimate_minutes',?) WHERE id=?")
            .bind(minutes)
            .bind(step_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE task_provenance SET estimate_minutes=? WHERE step_id=?")
            .bind(minutes)
            .bind(step_id)
            .execute(&mut *tx)
            .await?;
        if let Some(id) = task_id {
            let mut task: Task = sqlx::query_as("SELECT * FROM tasks WHERE id=?")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
            let before = task.clone();
            task.estimate_minutes = minutes;
            task.updated_at_utc = now.max(task.updated_at_utc + 1);
            crate::repository::write_task(&mut tx, &task).await?;
            crate::actions::record(&mut tx, Some(&before), &task, "estimate", now).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn goals(&self) -> Result<Vec<Goal>> {
        let rows: Vec<String> =
            sqlx::query_scalar("SELECT json FROM goals ORDER BY updated_at_utc DESC")
                .fetch_all(&self.pool)
                .await?;
        rows.into_iter()
            .map(|r| serde_json::from_str(&r).map_err(Into::into))
            .collect()
    }
    pub async fn save_goal(&self, goal: &Goal, now: i64) -> Result<()> {
        goal.validate()?;
        sqlx::query("INSERT INTO goals VALUES(?,?,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json,updated_at_utc=excluded.updated_at_utc").bind(&goal.id).bind(serde_json::to_string(goal)?).bind(now).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn steps(&self, goal_id: &str) -> Result<Vec<Step>> {
        let rows:Vec<(String,String,i64,String,Option<String>,bool)>=sqlx::query_as("SELECT id,goal_id,position,json,task_id,dismissed FROM goal_steps WHERE goal_id=? ORDER BY position").bind(goal_id).fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|(id, goal_id, position, data, task_id, dismissed)| {
                Ok(Step {
                    id,
                    goal_id,
                    position,
                    input: serde_json::from_str(&data)?,
                    task_id,
                    dismissed,
                })
            })
            .collect()
    }
    pub async fn dismiss_step(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE goal_steps SET dismissed=1 WHERE id=? AND task_id IS NULL")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn generate_steps(&self, goal_id: &str, now: i64) -> Result<Vec<Step>> {
        let goal = self
            .goals()
            .await?
            .into_iter()
            .find(|g| g.id == goal_id)
            .ok_or_else(|| Error::NotFound("Không có mục tiêu.".into()))?;
        let existing = self.steps(goal_id).await?;
        if !existing.is_empty() {
            return Ok(existing);
        }
        if goal.status != "active" {
            return Err(Error::Invalid("Mục tiêu đã tạm dừng/hoàn thành.".into()));
        }
        let settings = self.settings().await?;
        let ai=cloud::infer(&settings,"Chia mục tiêu trong context thành 3–5 bước, trả propose_goal_steps. Không tự chốt giờ; ghi đủ điều kiện phụ thuộc.",now,json!({"goal":goal})).await?;
        if ai.action != "propose_goal_steps" || !(3..=5).contains(&ai.tasks.len()) {
            return Err(Error::Invalid(
                "AI chưa trả 3–5 bước phù hợp; chưa lưu kế hoạch.".into(),
            ));
        }
        self.store_steps(goal_id, &ai.tasks).await?;
        self.steps(goal_id).await
    }
    pub async fn store_steps(&self, goal_id: &str, steps: &[AiTask]) -> Result<()> {
        let settings = self.settings().await?;
        cloud::validate_reply(
            &cloud::AiReply {
                action: "propose_goal_steps".into(),
                message: String::new(),
                tasks: steps.to_vec(),
            },
            &settings,
            chrono::Utc::now().timestamp(),
        )?;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM goal_steps WHERE goal_id=?")
            .bind(goal_id)
            .fetch_one(&mut *tx)
            .await?;
        if count > 0 {
            return Ok(());
        }
        for (i, step) in steps.iter().enumerate() {
            sqlx::query("INSERT INTO goal_steps(id,goal_id,position,json) VALUES(?,?,?,?)")
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(goal_id)
                .bind(i as i64)
                .bind(serde_json::to_string(step)?)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
    /// Interactive replan always proposes existing remaining steps. The engine
    /// uses auto_plan_once for authorized writes exactly once per local day.
    pub async fn plan(&self, date: NaiveDate, now: i64, _proposal: bool) -> Result<Value> {
        let mut proposals = vec![];
        for goal in self.goals().await?.iter().filter(|g| g.status == "active") {
            for step in self
                .steps(&goal.id)
                .await?
                .into_iter()
                .filter(|s| !s.dismissed && s.task_id.is_none())
            {
                proposals.push(json!({"goal":goal.title,"step":step,"date":date,"saved":false}));
            }
        }
        let _ = now;
        Ok(
            json!({"proposal":true,"date":date,"steps":proposals,"message":"Đề xuất từ các bước còn lại; chưa xóa/nhân đôi/đổi lịch task đã lưu. Chọn Thêm bước trong Mục tiêu để lưu."}),
        )
    }
    pub async fn add_goal_step(
        &self,
        goal_id: &str,
        step_id: &str,
        date: NaiveDate,
        now: i64,
    ) -> Result<Task> {
        self.add_step_with_source(goal_id, step_id, date, now, "goal")
            .await
    }
    async fn add_step_with_source(
        &self,
        goal_id: &str,
        step_id: &str,
        date: NaiveDate,
        now: i64,
        source: &str,
    ) -> Result<Task> {
        let _guard = self.mutation_gate.lock().await;
        let s = self.settings().await?;
        let goal = self
            .goals()
            .await?
            .into_iter()
            .find(|g| g.id == goal_id)
            .ok_or_else(|| Error::NotFound("Mục tiêu không tồn tại.".into()))?;
        let steps = self.steps(goal_id).await?;
        let step = steps
            .iter()
            .find(|x| x.id == step_id)
            .ok_or_else(|| Error::NotFound("Bước không tồn tại.".into()))?;
        if let Some(id) = &step.task_id {
            return self.get(id).await;
        }
        if step.dismissed || goal.status != "active" {
            return Err(Error::Invalid(
                "Bước đã loại hoặc mục tiêu đang dừng.".into(),
            ));
        }
        if let Some(index) = step.input.depends_on {
            let prerequisite = steps.get(index as usize).and_then(|s| s.task_id.as_ref());
            if let Some(id) = prerequisite {
                let t = self.get(id).await?;
                if t.status != "done" || t.archived_at_utc.is_some() {
                    return Err(Error::Invalid(
                        "Cần hoàn thành bước trước trước khi thêm bước phụ thuộc.".into(),
                    ));
                }
            } else {
                return Err(Error::Invalid("Bước trước chưa được thực hiện.".into()));
            }
        }
        let mut input = TaskInput::new(&step.input.title, None);
        input.description = step.input.description.clone();
        input.priority = step.input.priority.clone();
        input.tag_or_project = goal.title.chars().take(100).collect();
        crate::service::set_date(&mut input, Some(&date.to_string()), None, &s, now)?;
        input.validate()?;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let current: Option<String> =
            sqlx::query_scalar("SELECT task_id FROM goal_steps WHERE id=?")
                .bind(step_id)
                .fetch_one(&mut *tx)
                .await?;
        if let Some(id) = current {
            tx.rollback().await?;
            return self.get(&id).await;
        }
        let task = create_in(&mut tx, &input, now, &s.resolved_timezone()?).await?;
        sqlx::query("UPDATE goal_steps SET task_id=? WHERE id=?")
            .bind(&task.id)
            .bind(step_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO task_provenance(task_id,source,request_id,goal_id,step_id,original_request,estimate_minutes) VALUES(?,?,?,?,?,?,?)")
            .bind(&task.id).bind(source).bind(format!("step:{step_id}")).bind(goal_id).bind(step_id).bind(&goal.description).bind(step.input.estimate_minutes).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO task_events(task_id,action,after_json,at_utc) VALUES(?,'create',?,?)",
        )
        .bind(&task.id)
        .bind(serde_json::to_string(&task)?)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(task)
    }
    pub async fn auto_plan_once(&self, now: i64) -> Result<bool> {
        let settings = self.settings().await?;
        let local = settings.local_datetime(now);
        let date = local.date_naive();
        if !settings.auto_plan
            || !settings
                .work_days
                .contains(&local.weekday().num_days_from_monday())
            || local.hour() < 8
            || local.hour() >= 18
        {
            return Ok(false);
        }
        let goals: Vec<_> = self
            .goals()
            .await?
            .into_iter()
            .filter(|g| {
                g.status == "active"
                    && g.missing_information.trim().is_empty()
                    && g.auto_add
                    && g.start_date <= date.to_string()
                    && g.deadline.as_ref().is_none_or(|d| d >= &date.to_string())
                    && g.allowed_days
                        .contains(&local.weekday().num_days_from_monday())
            })
            .collect();
        if goals.is_empty() {
            return Ok(false);
        }
        let run: Option<(String, u32, i64)> = sqlx::query_as(
            "SELECT status,attempts,last_attempt_utc FROM plan_runs WHERE local_date=?",
        )
        .bind(date.to_string())
        .fetch_optional(&self.pool)
        .await?;
        if run.as_ref().is_some_and(|(s, n, t)| {
            s == "done" || *n >= settings.ai_daily_call_limit || now - *t < 300
        }) {
            return Ok(false);
        }
        if settings.ai_daily_call_limit == 0 || settings.ai_daily_task_limit == 0 {
            return Ok(false);
        }
        sqlx::query("INSERT INTO plan_runs VALUES(?,'running',1,?,NULL) ON CONFLICT(local_date) DO UPDATE SET status='running',attempts=attempts+1,last_attempt_utc=excluded.last_attempt_utc").bind(date.to_string()).bind(now).execute(&self.pool).await?;
        let result:Result<bool>=async {
            // Count against durable provenance, including tasks subsequently deleted.
            let start=crate::repository::local_stamp(&settings.resolved_timezone()?,date.and_hms_opt(0,0,0).unwrap())?;
            let used:i64=sqlx::query_scalar("SELECT count(*) FROM tasks t JOIN task_provenance p ON p.task_id=t.id WHERE p.source='planner' AND t.created_at_utc>=? AND t.created_at_utc<?").bind(start).bind(start+86400).fetch_one(&self.pool).await?;
            let mut budget=settings.ai_daily_task_limit.saturating_sub(used as u32);
            let mut minutes_left=((18*60)-(local.hour()*60+local.minute())) as i64;
            let tasks=self.list(false).await?;
            minutes_left-=tasks.iter().filter(|t|t.active()&&crate::scheduler::relevant(t,&settings,now)).count() as i64*30;
            let mut changed=false;
            for goal in goals {
                let steps=if self.steps(&goal.id).await?.is_empty(){
                    let calls:i64=sqlx::query_scalar("SELECT count(*) FROM ai_calls WHERE local_date=?").bind(date.to_string()).fetch_one(&self.pool).await?;
                    if calls>=settings.ai_daily_call_limit as i64{break;}
                    let key=uuid::Uuid::new_v4().to_string();
                    sqlx::query("INSERT INTO ai_calls VALUES(?,?,?,?,'started')").bind(&key).bind(date.to_string()).bind(&goal.id).bind(now).execute(&self.pool).await?;
                    let result=self.generate_steps(&goal.id,now).await;
                    sqlx::query("UPDATE ai_calls SET status=? WHERE id=?").bind(if result.is_ok(){"succeeded"}else{"failed"}).bind(key).execute(&self.pool).await?;
                    result?
                }else{self.steps(&goal.id).await?};
                for step in steps.into_iter().filter(|s|!s.dismissed&&s.task_id.is_none()){
                    if budget==0 {break;}
                    let estimate=step.input.estimate_minutes.unwrap_or(30) as i64;if estimate>minutes_left{continue;}
                    match self.add_step_with_source(&goal.id,&step.id,date,now,"planner").await{Ok(_)=>{budget-=1;minutes_left-=estimate;changed=true;},Err(Error::Invalid(_))=>continue,Err(e)=>return Err(e)}
                }
            }Ok(changed)
        }.await;
        sqlx::query("UPDATE plan_runs SET status=?,error=? WHERE local_date=?")
            .bind(if result.is_ok() { "done" } else { "failed" })
            .bind(
                result
                    .as_ref()
                    .err()
                    .map(|_| "AI/kế hoạch chưa sẵn sàng; xem cấu hình hoặc thử lại sau 5 phút"),
            )
            .bind(date.to_string())
            .execute(&self.pool)
            .await?;
        result
    }
}
