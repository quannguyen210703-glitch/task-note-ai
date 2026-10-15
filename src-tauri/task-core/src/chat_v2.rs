use crate::{
    actions::Action,
    assistant::{normalize, reply, Reply},
    cloud,
    repository::local_stamp,
    *,
};
use chrono::{Duration, NaiveDate, NaiveTime};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
fn re(s: &str) -> Regex {
    Regex::new(s).expect("static pattern")
}
#[derive(Clone, Serialize, Deserialize)]
struct Pending {
    text: String,
    received: i64,
    view_date: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Batch {
    pub inputs: Vec<TaskInput>,
    pub source: String,
    pub original: String,
    pub request_id: String,
    #[serde(default)]
    pub goal: Option<crate::goals::Goal>,
    #[serde(default)]
    pub steps: Vec<cloud::AiTask>,
}
pub fn offline_inputs(
    text: &str,
    s: &Settings,
    now: i64,
    view_date: Option<&str>,
) -> Result<Option<Vec<TaskInput>>> {
    let norm = normalize(text);
    let literal_task=re(r"^(?:(?:hom nay|ngay mai|mai)\s+)?(?:(?:anh|toi|em)\s+)?them(?:\s+mot)?\s+viec\b").is_match(&norm);
    if !literal_task && ["soan","viet nhap","tom tat","muc tieu","lap ke hoach"].iter().any(|v|norm.contains(v)){return Ok(None);}
    if norm.contains('?')
        || [
            "co nen",
            "dang nghi",
            "dang can nhac",
            "bao nhieu",
            "viec gi",
            "co viec",
            "goi y",
            "chia viec",
            "chia thanh",
        ]
        .iter()
        .any(|v| norm.contains(v))
        || norm.starts_with(['"', '“', '\''])
    {
        return Ok(None);
    }
    let has_create=re(r"^(?:(?:hom nay|ngay mai|mai|chieu mai)\s+)?(?:(?:anh|toi|em)\s+)?(?:them|nhac|can\s+lam)\b").is_match(&norm)
        || re(r"^(?:hom nay|ngay mai|mai|chieu mai)\b").is_match(&norm);
    if !has_create {
        return Ok(None);
    }
    let mut inputs = vec![];
    let mut inherited = s.local_datetime(now).date_naive();
    for part in text
        .split([',', ';', '\n'])
        .filter(|v| !v.trim().is_empty())
    {
        let n = normalize(part);
        let recurring=n.contains("moi thu")||n.contains("moi chu nhat")||n.contains("hang ngay")||n.contains("moi ngay")||n.contains("ngay lam viec");
        let recurrence=if recurring{crate::assistant::parse_time(part,s,now)?.ok_or_else(||Error::Invalid("Lịch lặp qua chat cần giờ cụ thể; có thể tạo lịch lặp cả ngày trong chi tiết công việc.".into()))?}else{(0,None)};
        if !recurring && (re(r"(?i)\bthứ\b").is_match(part) || re(r"\bthu\s+(?:hai|ba|tu|nam|sau|bay|[2-7])\b").is_match(&n) || n.contains("tuan sau") || n.contains("chu nhat")) {
            return Err(Error::Invalid(
                "Anh muốn ngày cụ thể nào (dd/mm/yyyy)? Chưa lưu batch.".into(),
            ));
        }
        if re(r"\b(?:cai do|viec nay|viec do)\b").is_match(&n) {
            return Err(Error::Invalid(
                "Anh muốn thêm nội dung việc nào? Chưa lưu batch.".into(),
            ));
        }
        if n.contains("ngay dang xem") {
            inherited = NaiveDate::parse_from_str(
                view_date
                    .ok_or_else(|| Error::Invalid("Anh đang xem ngày nào (YYYY-MM-DD)?".into()))?,
                "%Y-%m-%d",
            )
            .map_err(|_| Error::Invalid("Ngày đang xem không hợp lệ.".into()))?;
        } else if crate::assistant::says_tomorrow(part) {
            inherited = s.local_datetime(now).date_naive() + Duration::days(1);
        } else if n.contains("hom nay") {
            inherited = s.local_datetime(now).date_naive();
        }
        if let Some(c) = re(r"\b(\d{1,2})/(\d{1,2})/(\d{4})\b").captures(&n) {
            inherited = NaiveDate::from_ymd_opt(
                c[3].parse().unwrap_or(0),
                c[2].parse().unwrap_or(0),
                c[1].parse().unwrap_or(0),
            )
            .ok_or_else(|| Error::Invalid("Ngày không hợp lệ; chưa lưu batch.".into()))?;
        }
        if let Some(c) = re(r"\b(\d{4}-\d{2}-\d{2})\b").captures(&n) {
            inherited = NaiveDate::parse_from_str(&c[1], "%Y-%m-%d")
                .map_err(|_| Error::Invalid("Ngày không hợp lệ; chưa lưu batch.".into()))?;
        }
        if recurring{inherited=s.local_datetime(recurrence.0).date_naive();}
        let captures = re(r"\b(\d{1,2})(?:h|:)(\d{1,2})?\b").captures(&n).map(|c| {
            (
                c[1].parse::<u32>().unwrap_or(99),
                c.get(2)
                    .map_or(0, |m| m.as_str().parse::<u32>().unwrap_or(99)),
            )
        });
        let clock = match captures {
            Some((h, m)) => NaiveTime::from_hms_opt(h, m, 0)
                .ok_or_else(|| Error::Invalid("Giờ không hợp lệ; dùng 24h, ví dụ 16h30.".into()))?,
            None => NaiveTime::MIN,
        };
        let mut title = part.trim().to_owned();
        if crate::assistant::says_tomorrow(part){title=re(r"(?i)^\s*(?:chiều\s+)?mai\s+").replace(&title,"").into_owned();}
        for pattern in [
            r"(?i)\b(?:vào\s+)?ngày đang xem\b",
            r"(?i:\b(?:hôm nay|ngày mai|chiều mai|buổi chiều)\b)|\bmai\b",
            r"(?i)\b(?:mỗi thứ\s+(?:Hai|Ba|Tư|Năm|Sáu|Bảy)|mỗi Chủ nhật|mỗi ngày làm việc|ngày làm việc|hàng ngày|mỗi ngày)\b",
            r"\b\d{1,2}/\d{1,2}/\d{4}\b",
            r"\b\d{4}-\d{2}-\d{2}\b",
            r"(?i)(?:lúc\s+)?\b\d{1,2}(?:h|:)\d{0,2}\b",
            r"(?i)^\s*(?:(?:anh|tôi|em)\s+)?(?:thêm(?:\s+một)?(?:\s+việc)?(?:\s+nữa)?|nhắc(?:\s+anh|\s+tôi)?|cần làm)\s*",
            r"(?i)^\s*(?:vào|lúc|ngày)\s+",
        ] {
            title = re(pattern).replace_all(&title, "").into_owned();
        }
        title = title
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim_matches([' ', '.'])
            .to_owned();
        let mut input = TaskInput::new(
            title,
            Some(local_stamp(
                &s.resolved_timezone()?,
                inherited.and_time(clock),
            )?),
        );
        input.is_all_day = captures.is_none();
        input.recurrence_rule=recurrence.1;
        if re(r"\b(?:buoi chieu|chieu mai|chieu nay)\b").is_match(&n) {
            input.description = "Buổi chiều; chưa chốt giờ làm.".into();
        }
        if n.contains("uu tien cao") {
            input.priority = "high".into();
        }
        input.validate()?;
        inputs.push(input);
    }
    if inputs.is_empty() {
        return Err(Error::Invalid("Anh muốn thêm việc gì?".into()));
    }
    Ok(Some(inputs))
}
impl Repository {
    pub async fn chat_v2(
        &self,
        text: &str,
        id: &str,
        received: i64,
        view_date: Option<&str>,
        dry_run: bool,
    ) -> Result<Reply> {
        if text.trim().is_empty() || text.chars().count() > 2000 || id.is_empty() || id.len() > 120
        {
            return Err(Error::Invalid(
                "Tin nhắn cần 1–2000 ký tự và request ID hợp lệ.".into(),
            ));
        }
        let stored:Option<(String,i64,String,Option<String>,Option<String>)>=sqlx::query_as("SELECT original_text,received_at_utc,context_json,reply_json,pending_json FROM chat_requests WHERE request_id=?").bind(id).fetch_optional(&self.pool).await?;
        let (mut actual, mut at, mut view) =
            (text.to_owned(), received, view_date.map(str::to_owned));
        if let Some((original, ts, context, saved, pending)) = stored {
            at = ts;
            view = serde_json::from_str::<serde_json::Value>(&context)
                .ok()
                .and_then(|v| v["view_date"].as_str().map(str::to_owned));
            if let Some(saved) = saved {
                if original == text {
                    return Ok(serde_json::from_str(&saved)?);
                }
            }
            if let Some(pending) = pending {
                let p: Pending = serde_json::from_str(&pending)?;
                at = p.received;
                view = p.view_date;
                if original != text {
                    let date = Regex::new(r"\d{1,2}/\d{1,2}/\d{4}|\d{4}-\d{2}-\d{2}")
                        .unwrap()
                        .find(text)
                        .map(|v| v.as_str().to_owned());
                    if let Some(date) = date {
                        actual = re(
                            r"(?i)(?:vào\s+)?(?:thứ\s+(?:Hai|Ba|Tư|Năm|Sáu|Bảy)|tuần sau|Chủ nhật)",
                        )
                        .replace_all(&p.text, &date)
                        .into_owned();
                    } else {
                        actual = format!("{} {}", p.text, text);
                    }
                } else {
                    actual = p.text;
                }
            } else if original != text {
                return Err(Error::Invalid(
                    "Request ID đã dùng cho tin nhắn khác.".into(),
                ));
            }
        }
        let s = self.settings().await?;
        let stale_context =
            s.local_datetime(received).date_naive() != s.local_datetime(at).date_naive();
        if !dry_run {
            if let Some(result) = sqlx::query_scalar::<_, String>(
                "SELECT result_json FROM mutation_requests WHERE request_id=?",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            {
                let outcome: crate::actions::Outcome = serde_json::from_str(&result)?;
                let mut result = reply(outcome.message);
                result.saved = outcome.tasks;
                result.request_id = Some(id.into());
                return Ok(result);
            }
        }
        if normalize(&actual).contains("them mot viec nua")
            && !re(r"hom nay|mai|\d{1,2}/\d{1,2}|\d{4}-\d{2}-\d{2}").is_match(&normalize(&actual))
        {
            let previous:Option<String>=sqlx::query_scalar("SELECT reply_json FROM chat_requests WHERE request_id<>? AND reply_json IS NOT NULL ORDER BY received_at_utc DESC LIMIT 1").bind(id).fetch_optional(&self.pool).await?;
            if let Some(previous) = previous.and_then(|v| serde_json::from_str::<Reply>(&v).ok()) {
                let days: std::collections::HashSet<_> = previous
                    .saved
                    .iter()
                    .filter_map(|t| t.due_at_utc.map(|u| s.local_datetime(u).date_naive()))
                    .collect();
                if days.len() == 1 {
                    actual = format!("{} {}", actual, days.iter().next().unwrap());
                }
            }
        }
        if !dry_run {
            sqlx::query("INSERT INTO chat_requests(request_id,original_text,received_at_utc,context_json) VALUES(?,?,?,?) ON CONFLICT DO NOTHING").bind(id).bind(text).bind(at).bind(json!({"view_date":view,"timezone":s.timezone}).to_string()).execute(&self.pool).await?;
        }
        let norm = normalize(&actual);
        let mut result;
        let inputs = match offline_inputs(&actual, &s, at, view.as_deref()) {
            Ok(Some(v)) => Some(v),
            Err(e) => {
                result = reply(e.to_string());
                result.pending = true;
                result.request_id = Some(id.into());
                if !dry_run {
                    sqlx::query("UPDATE chat_requests SET pending_json=? WHERE request_id=?")
                        .bind(serde_json::to_string(&Pending {
                            text: actual,
                            received: at,
                            view_date: view,
                        })?)
                        .bind(id)
                        .execute(&self.pool)
                        .await?;
                }
                return Ok(result);
            }
            Ok(None) => None,
        };
        if let Some(inputs) = inputs {
            result = self
                .save_or_preview(inputs, "chat", &actual, id, at, dry_run, stale_context)
                .await?;
        } else if norm.starts_with("da xong ")
            || norm.starts_with("hoan thanh ")
            || norm.starts_with("xoa ")
            || norm.starts_with("doi ")
            || norm.starts_with("doi lich ")
            || norm.starts_with("doi viec ")
        {
            result = if dry_run {
                reply("Lệnh thay đổi công việc cũ cần preview/xác nhận trong Trợ lý.")
            } else {
                self.chat(&actual, at).await?
            };
        } else if norm.contains("bao nhieu")
            || norm.contains("viec gi")
            || norm.contains("con viec")
        {
            let tasks = self.list(false).await?;
            let active: Vec<_> = tasks
                .iter()
                .filter(|t| t.active() && crate::scheduler::relevant(t, &s, at))
                .collect();
            result = reply(format!(
                "Hôm nay & quá hạn: {} việc chưa xong.\n{}",
                active.len(),
                active
                    .iter()
                    .map(|t| format!("• {}", t.title))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        } else if norm.contains("co nen")
            || norm.contains("dang can nhac")
            || norm.contains("dang nghi")
            || actual.starts_with(['"', '“', '\''])
        {
            result=reply("Chưa tạo việc từ nội dung thảo luận/trích dẫn. Anh có thể nói ‘Thêm việc …’ khi đã quyết định.");
        } else {
            let ai = cloud::infer(&s, &actual, at, json!({})).await?;
            match ai.action.as_str() {
                "create_tasks" => {
                    let inputs = cloud::validate_reply(&ai, &s, at)?;
                    // A cloud response alone cannot grant write permission.
                    let explicit = ![
                        "dung them",
                        "khong them",
                        "chi goi y",
                        "chi soan",
                        "viet nhap",
                    ]
                    .iter()
                    .any(|n| norm.contains(n))
                        && (norm.contains("them")
                            || norm.contains("anh can")
                            || norm.starts_with("nhac "));
                    result = self
                        .save_or_preview(
                            inputs,
                            "chat",
                            &actual,
                            id,
                            at,
                            dry_run,
                            !explicit || stale_context,
                        )
                        .await?;
                }
                "propose_goal_steps" => {
                    let explicit = norm.contains("them vao")
                        || norm.contains("chia viec cho")
                        || norm.contains("chia thanh viec va them");
                    let inputs = cloud::validate_reply(&ai, &s, at)?;
                    let recurring = norm.contains("tu lap viec") && norm.contains("moi ngay");
                    let goal = crate::goals::Goal {
                        id: uuid::Uuid::new_v4().to_string(),
                        title: actual.chars().take(300).collect(),
                        description: actual.clone(),
                        desired_outcome: String::new(),
                        start_date: s.local_datetime(at).date_naive().to_string(),
                        deadline: None,
                        allowed_days: s.work_days.clone(),
                        auto_add: recurring,
                        status: "active".into(),
                        missing_information: String::new(),
                    };
                    let action = Action::GoalBatch {
                        goal: goal.clone(),
                        steps: ai.tasks.clone(),
                        original: actual.clone(),
                    };
                    if recurring && !dry_run {
                        let out = self
                            .mutate(
                                &Action::ConfigureGoal {
                                    goal,
                                    steps: ai.tasks.clone(),
                                },
                                id,
                                at,
                                false,
                            )
                            .await?;
                        result = reply(out.message);
                    } else if explicit
                        && !dry_run
                        && !stale_context
                        && !inputs.iter().any(|i| {
                            i.due_at_utc.is_some_and(|u| {
                                if i.is_all_day {
                                    s.local_datetime(u).date_naive()
                                        < s.local_datetime(at).date_naive()
                                } else {
                                    u < at
                                }
                            })
                        })
                    {
                        let out = self.mutate(&action, id, at, false).await?;
                        result = reply(out.message);
                        result.saved = out.tasks;
                    } else {
                        result = reply("Đề xuất các bước của mục tiêu; chưa lưu việc.");
                        result.saved = inputs
                            .iter()
                            .map(|i| crate::repository::task_from_input(i, at))
                            .collect();
                        if !dry_run {
                            let token = uuid::Uuid::new_v4().to_string();
                            sqlx::query("INSERT INTO batch_proposals(token,payload_json,created_at_utc) VALUES(?,?,?)").bind(&token).bind(serde_json::to_string(&Batch{inputs,source:"goal".into(),original:actual.clone(),request_id:id.into(),goal:Some(goal),steps:ai.tasks})?).bind(at).execute(&self.pool).await?;
                            result.batch_token = Some(token);
                        }
                    }
                }
                _ => result = reply(ai.message),
            }
        }
        result.request_id = Some(id.into());
        if !dry_run {
            sqlx::query(
                "UPDATE chat_requests SET reply_json=?,pending_json=NULL WHERE request_id=?",
            )
            .bind(serde_json::to_string(&result)?)
            .bind(id)
            .execute(&self.pool)
            .await?;
            let _ = self.save_chat("user", text, at).await;
            let _ = self.save_chat("assistant", &result.text, at).await;
        }
        Ok(result)
    }
    async fn save_or_preview(
        &self,
        inputs: Vec<TaskInput>,
        source: &str,
        original: &str,
        id: &str,
        at: i64,
        dry_run: bool,
        proposal: bool,
    ) -> Result<Reply> {
        let s = self.settings().await?;
        let tasks = self.list(false).await?;
        let duplicate = inputs.iter().any(|i| {
            tasks
                .iter()
                .any(|t| normalize(&t.title) == normalize(&i.title) && t.due_at_utc == i.due_at_utc)
        });
        let past = inputs.iter().any(|i| {
            i.due_at_utc.is_some_and(|u| {
                if i.is_all_day {
                    s.local_datetime(u).date_naive() < s.local_datetime(at).date_naive()
                } else {
                    u < at
                }
            })
        });
        if dry_run || proposal || duplicate || past {
            let mut result = reply(if dry_run {
                "Dự kiến; chưa lưu batch."
            } else if duplicate {
                "Có việc giống tên và thời điểm. Thêm bản nữa?"
            } else if past {
                "Thời điểm đã qua/ngày gửi đã đổi. Giữ ngày giờ dự kiến hay gửi lại yêu cầu?"
            } else {
                "Đề xuất; chưa thêm vào lịch."
            });
            result.saved = inputs
                .iter()
                .map(|i| crate::repository::task_from_input(i, at))
                .collect();
            if !dry_run {
                let token = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO batch_proposals(token,payload_json,created_at_utc) VALUES(?,?,?)",
                )
                .bind(&token)
                .bind(serde_json::to_string(&Batch {
                    inputs,
                    source: source.into(),
                    original: original.into(),
                    request_id: id.into(),
                    goal: None,
                    steps: vec![],
                })?)
                .bind(at)
                .execute(&self.pool)
                .await?;
                result.batch_token = Some(token);
            }
            return Ok(result);
        }
        let outcome = self
            .mutate(
                &Action::Add {
                    inputs,
                    source: source.into(),
                    original: original.into(),
                },
                id,
                at,
                false,
            )
            .await?;
        let mut result = reply(format!(
            "Đã thêm {} việc: {}",
            outcome.tasks.len(),
            outcome
                .tasks
                .iter()
                .map(|t| format!(
                    "{} ({})",
                    t.title,
                    s.local_datetime(t.due_at_utc.unwrap_or(at))
                        .format(if t.is_all_day {
                            "%d/%m/%Y · cả ngày"
                        } else {
                            "%d/%m/%Y %H:%M"
                        })
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ));
        result.saved = outcome.tasks;
        result.request_id = Some(id.into());
        Ok(result)
    }
    pub async fn confirm_batch(&self, token: &str, now: i64) -> Result<crate::actions::Outcome> {
        let (payload, created): (String, i64) =
            sqlx::query_as("SELECT payload_json,created_at_utc FROM batch_proposals WHERE token=?")
                .bind(token)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| Error::NotFound("Không tìm thấy đề xuất.".into()))?;
        if now < created || now - created > 86400 {
            return Err(Error::Invalid(
                "Đề xuất đã hết hạn; hãy gửi lại để kiểm tra ngày.".into(),
            ));
        }
        let batch: Batch = serde_json::from_str(&payload)?;
        let action = if let Some(goal) = batch.goal {
            Action::GoalBatch {
                goal,
                steps: batch.steps,
                original: batch.original,
            }
        } else {
            Action::Add {
                inputs: batch.inputs,
                source: batch.source,
                original: batch.original,
            }
        };
        let outcome = self.mutate(&action, &batch.request_id, now, false).await?;
        // The receipt is atomic with task writes; proposal bookkeeping may retry.
        sqlx::query("UPDATE batch_proposals SET consumed_at_utc=? WHERE token=?")
            .bind(now)
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(outcome)
    }
}
