use crate::{
    model::*,
    repository::{create_in, local_stamp, write_task},
    Repository,
};
use chrono::{Datelike, Duration, NaiveDate};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn normalize(s: &str) -> String {
    deunicode::deunicode(s).to_lowercase()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub action: String,
    pub input: TaskInput,
    pub task_id: Option<String>,
    pub expected: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalCard {
    pub token: String,
    pub label: String,
    pub input: TaskInput,
    pub action: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply {
    pub text: String,
    pub proposals: Vec<ProposalCard>,
    #[serde(default)]
    pub saved: Vec<Task>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub batch_token: Option<String>,
    #[serde(default)]
    pub pending: bool,
}
pub fn reply(text: impl Into<String>) -> Reply {
    Reply {
        text: text.into(),
        proposals: vec![],
        saved: vec![],
        request_id: None,
        batch_token: None,
        pending: false,
    }
}
fn regex(s: &str) -> Regex {
    Regex::new(s).expect("static regex")
}
pub fn says_tomorrow(text:&str)->bool {
    regex(r"\bngay mai\b").is_match(&normalize(text))
        || regex(r"(?i)^\s*(?:chiều\s+)?mai\b").is_match(text)
        || regex(r"\bmai\b").is_match(text)
}
pub fn parse_time(text: &str, s: &Settings, now: i64) -> Result<Option<(i64, Option<String>)>> {
    let norm = normalize(text);
    let local = s.local_datetime(now);
    let mut date = local.date_naive();
    let time = regex(r"(?:^|\s)(\d{1,2})(?:h|:)(\d{1,2})?(?:\b|\s|$)").captures(&norm);
    let Some(time) = time else { return Ok(None) };
    let hour: u32 = time[1].parse().unwrap();
    let minute: u32 = time.get(2).map_or(0, |m| m.as_str().parse().unwrap_or(99));
    let Some(clock) = chrono::NaiveTime::from_hms_opt(hour, minute, 0) else {
        return Err(Error::Invalid(
            "Giờ chưa hợp lệ. Dùng giờ 24h, ví dụ 14h30.".into(),
        ));
    };
    if norm.contains("tuan sau") {
        return Err(Error::Invalid(
            "Bạn muốn ngày nào tuần sau? Dùng ngày dd/mm/yyyy và giờ cụ thể.".into(),
        ));
    }
    let weekly = regex(r"moi thu (hai|ba|tu|nam|sau|bay)").captures(&norm);
    let mut recurrence = None;
    if let Some(w) = weekly {
        let day = match &w[1] {
            "hai" => 0,
            "ba" => 1,
            "tu" => 2,
            "nam" => 3,
            "sau" => 4,
            _ => 5,
        };
        let delta = (day + 7 - date.weekday().num_days_from_monday()) % 7;
        date += Duration::days(delta as i64);
        if delta == 0 && clock <= local.time() {
            date += Duration::days(7);
        }
        recurrence = Some(format!("weekly:{day}"));
    } else if norm.contains("moi chu nhat") {
        let delta = (6 + 7 - date.weekday().num_days_from_monday()) % 7;
        date += Duration::days(delta as i64);
        if delta == 0 && clock <= local.time() {
            date += Duration::days(7);
        }
        recurrence = Some("weekly:6".into());
    } else if regex(r"\bthu\s+(?:hai|ba|tu|nam|sau|bay|[2-7])\b").is_match(&norm) || norm.contains("chu nhat") {
        return Err(Error::Invalid(
            "Bạn muốn ngày cụ thể nào? Dùng dd/mm/yyyy, hoặc ‘mỗi thứ …’ cho lịch lặp.".into(),
        ));
    } else if says_tomorrow(text) {
        date += Duration::days(1);
    }
    let explicit = regex(r"\b(\d{1,2})/(\d{1,2})/(\d{4})\b");
    if let Some(c) = explicit.captures(&norm) {
        date = NaiveDate::from_ymd_opt(
            c[3].parse().unwrap_or(0),
            c[2].parse().unwrap_or(0),
            c[1].parse().unwrap_or(0),
        )
        .ok_or_else(|| Error::Invalid("Ngày không hợp lệ.".into()))?;
    }
    if norm.contains("hang ngay") || norm.contains("moi ngay") {
        recurrence = Some("daily".into());
    }
    if norm.contains("ngay lam viec") {
        recurrence = Some("weekdays".into());
        while date.weekday().num_days_from_monday() >= 5 {
            date += Duration::days(1);
        }
    }
    Ok(Some((
        local_stamp(&s.timezone, date.and_time(clock))?,
        recurrence,
    )))
}
fn clean_title(text: &str) -> String {
    let prefix = regex(r"(?i)^(?:thêm(?:\s+việc)?|nhắc(?:\s+anh|\s+em|\s+tôi)?|mai|ngày mai)\s+");
    let time = regex(r"(?i)(?:lúc\s+)?\b\d{1,2}(?:h|:)\d{0,2}\b");
    let date = regex(
        r"(?i)(?:hôm nay|ngày mai|\bmai\b|\d{1,2}/\d{1,2}/\d{4}|(?:hàng ngày|mỗi ngày|mỗi ngày làm việc)|mỗi thứ\s+(?:Hai|Ba|Tư|Năm|Sáu|Bảy)|mỗi Chủ nhật)",
    );
    let raw = prefix.replace(text.trim(), "");
    let raw = time.replace_all(&raw, "");
    let raw = date.replace_all(&raw, "");
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches([' ', '.', '?'])
        .to_string()
}
impl Repository {
    pub async fn propose(&self, proposal: Proposal, now: i64) -> Result<ProposalCard> {
        proposal.input.validate()?;
        let token = Uuid::new_v4().to_string();
        sqlx::query("DELETE FROM proposals WHERE created_at_utc<?")
            .bind(now - 86400)
            .execute(&self.pool)
            .await?;
        sqlx::query("INSERT INTO proposals(token,payload,created_at_utc) VALUES(?,?,?)")
            .bind(&token)
            .bind(serde_json::to_string(&proposal)?)
            .bind(now)
            .execute(&self.pool)
            .await?;
        let label = match proposal.action.as_str() {
            "create" => "Thêm công việc",
            "done" => "Hoàn thành",
            "delete" => "Đưa vào thùng rác",
            _ => "Đổi lịch",
        }
        .into();
        Ok(ProposalCard {
            token,
            label,
            input: proposal.input,
            action: proposal.action,
        })
    }
    pub async fn chat(&self, text: &str, now: i64) -> Result<Reply> {
        if text.trim().is_empty() || text.chars().count() > 2000 {
            return Err(Error::Invalid("Tin nhắn cần 1–2.000 ký tự.".into()));
        }
        let norm = normalize(text);
        let settings = self.settings().await?;
        let tasks = self.list(false).await?;
        let result = if norm.contains("bao nhieu") || norm.contains("chua lam") {
            reply(format!(
                "Bạn còn {} việc chưa hoàn thành ({} đang làm).",
                tasks.iter().filter(|t| t.active()).count(),
                tasks.iter().filter(|t| t.status == "doing").count()
            ))
        } else if norm.contains("hom nay")
            && (norm.contains("viec gi") || norm.contains("co viec") || norm.starts_with("xem"))
        {
            let related: Vec<_> = tasks
                .iter()
                .filter(|t| t.active() && crate::scheduler::relevant(t, &settings, now))
                .collect();
            reply(if related.is_empty() {
                "Hôm nay chưa có việc đang chờ. Bạn có thể thêm việc mới.".into()
            } else {
                format!(
                    "Hôm nay và quá hạn: {} việc\n{}",
                    related.len(),
                    related
                        .iter()
                        .map(|t| format!(
                            "• {}{}",
                            t.title,
                            if crate::scheduler::overdue(t, &settings, now) {
                                " (quá hạn)"
                            } else {
                                ""
                            }
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
        } else if norm.starts_with("da xong ")
            || norm.starts_with("hoan thanh ")
            || norm.starts_with("doi ")
            || norm.starts_with("doi viec ")
            || norm.starts_with("doi lich ")
            || norm.starts_with("doi")
            || norm.starts_with("dời")
            || norm.starts_with("xoa ")
        {
            let action = if norm.starts_with("da xong") || norm.starts_with("hoan thanh") {
                "done"
            } else if norm.starts_with("xoa") {
                "delete"
            } else {
                "move"
            };
            let target =
                regex(r"(?i)^(?:đã xong|hoàn thành|dời(?: việc)?|đổi(?: lịch)?|xóa(?: việc)?)\s+")
                    .replace(text, "");
            let target = regex(r"(?i)\s+sang\s+.*$")
                .replace(&target, "")
                .trim()
                .to_string();
            let target = normalize(&target);
            let matching: Vec<_> = tasks
                .iter()
                .filter(|t| t.active() && normalize(&t.title).contains(&target))
                .collect();
            if target.is_empty() || matching.is_empty() {
                reply("Chưa tìm thấy công việc. Hãy dùng tên rõ hơn.")
            } else if matching.len() > 12 {
                reply(
                    "Có nhiều việc trùng tên. Hãy dùng tên chi tiết hơn hoặc chọn trong Dashboard.",
                )
            } else {
                let parsed = if action == "move" {
                    match parse_time(text,&settings,now){Ok(Some(t))=>Some(t),Ok(None)=>return Ok(reply("Bạn muốn dời sang ngày và giờ nào? Ví dụ ‘Dời việc báo cáo sang 16h hôm nay’.")),Err(e)=>return Ok(reply(e.to_string()))}
                } else {
                    None
                };
                let mut cards = Vec::new();
                for task in matching {
                    let mut input = task.input();
                    if action == "done" {
                        input.status = "done".into();
                    }
                    if let Some((due, _)) = parsed {
                        input.due_at_utc = Some(due);
                    }
                    cards.push(
                        self.propose(
                            Proposal {
                                action: action.into(),
                                input,
                                task_id: Some(task.id.clone()),
                                expected: Some(task.updated_at_utc),
                            },
                            now,
                        )
                        .await?,
                    );
                }
                Reply {
                    text: if cards.len() > 1 {
                        "Có nhiều công việc phù hợp. Chỉ xác nhận đúng công việc bạn muốn."
                    } else {
                        "Kiểm tra đề xuất; chưa thay đổi công việc."
                    }
                    .into(),
                    proposals: cards,
                    saved: vec![],
                    request_id: None,
                    batch_token: None,
                    pending: false,
                }
            }
        } else {
            match parse_time(text,&settings,now) {
                Ok(Some((due,rule)))=>{let title=clean_title(text);if title.is_empty(){reply("Bạn muốn thêm việc gì vào thời điểm này?")}else{let mut input=TaskInput::new(title,Some(due));input.recurrence_rule=rule;Reply{text:"Đề xuất dưới đây chưa được lưu. Kiểm tra ngày, giờ và xác nhận.".into(),proposals:vec![self.propose(Proposal{action:"create".into(),input,task_id:None,expected:None},now).await?],..reply("")}}},
                Ok(None)=>reply("Tôi hiểu lệnh offline: xem việc hôm nay, đếm việc, thêm việc, dời lịch, hoàn thành hoặc xóa. Ví dụ ‘Mai 9h gọi khách hàng’. Nếu thêm việc theo lịch, hãy ghi giờ cụ thể."),
                Err(e)=>reply(e.to_string()),
            }
        };
        self.save_chat("user", text, now).await?;
        self.save_chat("assistant", &result.text, now).await?;
        Ok(result)
    }
    pub async fn confirm(&self, token: &str, now: i64) -> Result<Task> {
        let _guard = self.mutation_gate.lock().await;
        let zone = self.settings().await?.resolved_timezone()?;
        let mut tx = self.pool.begin().await?;
        let row: Option<(String, i64, Option<i64>)> = sqlx::query_as(
            "SELECT payload,created_at_utc,consumed_at_utc FROM proposals WHERE token=?",
        )
        .bind(token)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((json, created, consumed)) = row else {
            return Err(Error::Invalid("Đề xuất không tồn tại.".into()));
        };
        if consumed.is_some() || now - created > 600 || now < created {
            return Err(Error::Invalid(
                "Đề xuất đã dùng hoặc hết hạn sau 10 phút. Hãy gửi lệnh lại.".into(),
            ));
        }
        let proposal: Proposal = serde_json::from_str(&json)?;
        proposal.input.validate()?;
        let task = if proposal.action == "create" {
            create_in(&mut tx, &proposal.input, now, &zone).await?
        } else {
            let mut task: Task =
                sqlx::query_as("SELECT * FROM tasks WHERE id=? AND archived_at_utc IS NULL")
                    .bind(&proposal.task_id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or_else(|| Error::Invalid("Không tìm thấy công việc.".into()))?;
            if Some(task.updated_at_utc) != proposal.expected {
                return Err(Error::Invalid(
                    "Công việc đã thay đổi. Hãy gửi lệnh lại để xem đề xuất mới.".into(),
                ));
            }
            let before = task.clone();
            if proposal.action == "done" {
                task.status = "done".into();
                task.completed_at_utc = Some(now);
            }
            if proposal.action == "move" {
                task.due_at_utc = proposal.input.due_at_utc;
                task.snoozed_until_utc = None;
            }
            if proposal.action == "delete" {
                task.archived_at_utc = Some(now);
            }
            task.updated_at_utc = now.max(task.updated_at_utc + 1);
            write_task(&mut tx, &task).await?;
            crate::actions::record(&mut tx, Some(&before), &task, &proposal.action, now).await?;
            task
        };
        sqlx::query(
            "UPDATE proposals SET consumed_at_utc=? WHERE token=? AND consumed_at_utc IS NULL",
        )
        .bind(now)
        .bind(token)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        if task.template_id.is_some() {
            self.materialize(now).await?;
        }
        Ok(task)
    }
}
// Cloud adapter is deliberately separate and OFF in V1; no network calls or API-key UI.
pub trait OptionalCloudAIProvider {
    fn is_enabled(&self) -> bool;
}
