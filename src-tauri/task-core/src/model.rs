use chrono::{DateTime, Local, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Cơ sở dữ liệu: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Migration: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("Định dạng dữ liệu: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Ambiguous(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub due_at_utc: Option<i64>,
    pub is_all_day: bool,
    pub priority: String,
    pub status: String,
    pub tag_or_project: String,
    pub recurrence_rule: Option<String>,
    pub snoozed_until_utc: Option<i64>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub completed_at_utc: Option<i64>,
    pub archived_at_utc: Option<i64>,
    pub template_id: Option<String>,
    pub occurrence_date: Option<String>,
    #[serde(default)]
    #[sqlx(default)]
    pub local_date: Option<String>,
    #[serde(default)]
    #[sqlx(default)]
    pub task_timezone: Option<String>,
    #[serde(default)]
    #[sqlx(default)]
    pub source: Option<String>,
    #[serde(default)]
    #[sqlx(default)]
    pub goal_id: Option<String>,
    #[serde(default)]
    #[sqlx(default)]
    pub estimate_minutes: Option<i64>,
    #[serde(default)]
    #[sqlx(default)]
    pub blocked: bool,
}
impl Task {
    pub fn local_day(&self,settings:&Settings)->Option<chrono::NaiveDate>{
        if self.is_all_day {
            if let Some(day)=self.local_date.as_deref().and_then(|d|chrono::NaiveDate::parse_from_str(d,"%Y-%m-%d").ok()){return Some(day);}
        }
        self.due_at_utc.map(|u|settings.local_datetime(u).date_naive())
    }
    pub fn effective_due(&self) -> Option<i64> {
        self.snoozed_until_utc.or(self.due_at_utc)
    }
    pub fn active(&self) -> bool {
        self.archived_at_utc.is_none() && self.status != "done"
    }
    pub fn input(&self) -> TaskInput {
        TaskInput {
            title: self.title.clone(),
            description: self.description.clone(),
            due_at_utc: self.due_at_utc,
            is_all_day: self.is_all_day,
            priority: self.priority.clone(),
            status: self.status.clone(),
            tag_or_project: self.tag_or_project.clone(),
            recurrence_rule: self.recurrence_rule.clone(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInput {
    pub title: String,
    pub description: String,
    pub due_at_utc: Option<i64>,
    pub is_all_day: bool,
    pub priority: String,
    pub status: String,
    pub tag_or_project: String,
    pub recurrence_rule: Option<String>,
}
impl TaskInput {
    pub fn new(title: impl Into<String>, due: Option<i64>) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            due_at_utc: due,
            is_all_day: false,
            priority: "medium".into(),
            status: "todo".into(),
            tag_or_project: String::new(),
            recurrence_rule: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.title.trim().is_empty()
            || self.title.chars().count() > 300
            || self.description.chars().count() > 10000
            || self.tag_or_project.chars().count() > 100
        {
            return Err(Error::Invalid(
                "Tiêu đề 1–300 ký tự; ghi chú tối đa 10.000; tag tối đa 100.".into(),
            ));
        }
        if !["high", "medium", "low"].contains(&self.priority.as_str())
            || !["todo", "doing", "done"].contains(&self.status.as_str())
        {
            return Err(Error::Invalid(
                "Trạng thái hoặc ưu tiên không hợp lệ.".into(),
            ));
        }
        if let Some(due) = self.due_at_utc {
            if DateTime::from_timestamp(due, 0).is_none() || !(0..=4102444800).contains(&due) {
                return Err(Error::Invalid("Ngày hẹn phải thuộc 1970–2100.".into()));
            }
        }
        if let Some(rule) = &self.recurrence_rule {
            if !valid_rule(rule) || self.due_at_utc.is_none() {
                return Err(Error::Invalid(
                    "Lịch lặp cần ngày bắt đầu và quy tắc daily, weekdays hoặc weekly:0..6.".into(),
                ));
            }
        }
        Ok(())
    }
}
pub fn valid_rule(rule: &str) -> bool {
    rule == "daily"
        || rule == "weekdays"
        || rule
            .strip_prefix("weekly:")
            .is_some_and(|s| s.parse::<u32>().is_ok_and(|n| n < 7))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub onboarding_done: bool,
    pub autostart: bool,
    pub show_sticky: bool,
    pub pinned: bool,
    pub notifications: bool,
    pub hourly: bool,
    pub pre_alert: bool,
    pub due_alert: bool,
    pub daily_summary: bool,
    pub sound: bool,
    pub paused: bool,
    pub pause_until_utc: Option<i64>,
    pub ai_enabled: bool,
    pub ai_consent: bool,
    pub ai_provider: String,
    pub ai_model: String,
    pub auto_plan: bool,
    pub ai_daily_task_limit: u32,
    pub ai_daily_call_limit: u32,
    pub work_days: Vec<u32>,
    pub pre_minutes: i64,
    pub timezone: String,
    pub theme: String,
    pub save_chat: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            onboarding_done: false,
            autostart: false,
            show_sticky: true,
            pinned: true,
            notifications: true,
            hourly: true,
            pre_alert: true,
            due_alert: true,
            daily_summary: true,
            sound: true,
            paused: false,
            pause_until_utc: None,
            ai_enabled: false,
            ai_consent: false,
            ai_provider: "openai".into(),
            ai_model: String::new(),
            auto_plan: false,
            ai_daily_task_limit: 3,
            ai_daily_call_limit: 3,
            work_days: vec![0, 1, 2, 3, 4],
            pre_minutes: 15,
            timezone: "Asia/Ho_Chi_Minh".into(),
            theme: "light".into(),
            save_chat: false,
        }
    }
}
impl Settings {
    pub fn resolved_timezone(&self) -> Result<String> {
        if self.timezone == "system" {
            iana_time_zone::get_timezone()
                .map_err(|e| Error::Invalid(format!("Không đọc được múi giờ Windows: {e}")))
        } else {
            Ok(self.timezone.clone())
        }
    }
    pub fn zone(&self) -> Result<Tz> {
        self.resolved_timezone()?
            .parse()
            .map_err(|_| Error::Invalid("Múi giờ IANA không hợp lệ.".into()))
    }
    pub fn local_datetime(&self, ts: i64) -> DateTime<chrono::FixedOffset> {
        let utc = DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
        if self.timezone == "system" {
            utc.with_timezone(&Local).fixed_offset()
        } else {
            utc.with_timezone(&self.zone().unwrap_or(chrono_tz::Asia::Ho_Chi_Minh))
                .fixed_offset()
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.zone()?;
        if !["openai", "codex"].contains(&self.ai_provider.as_str())
            || self.ai_model.len() > 120
            || self.ai_model.chars().any(char::is_control)
            || self.ai_daily_task_limit > 20
            || self.ai_daily_call_limit > 20
            || self.work_days.iter().any(|d| *d > 6)
        {
            return Err(Error::Invalid(
                "Cấu hình AI/lịch làm việc không hợp lệ.".into(),
            ));
        }
        if !(1..=120).contains(&self.pre_minutes)
            || !["light", "dark"].contains(&self.theme.as_str())
        {
            return Err(Error::Invalid("Cài đặt không hợp lệ.".into()));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Template {
    pub id: String,
    pub input_json: String,
    pub timezone: String,
    pub start_date: String,
    pub generated_until: Option<String>,
    pub archived_at_utc: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    pub version: u32,
    pub exported_at_utc: i64,
    pub tasks: Vec<Task>,
    pub templates: Vec<Template>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ImportPreview {
    pub new_tasks: usize,
    pub duplicate_tasks: usize,
    pub templates: usize,
}
