use crate::{Settings, Task};
use chrono::Timelike;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Notice {
    pub key: String,
    pub kind: String,
    pub title: String,
    pub body: String,
}
pub trait Clock {
    fn now(&self) -> i64;
}
pub struct WallClock;
impl Clock for WallClock {
    fn now(&self) -> i64 {
        chrono::Utc::now().timestamp()
    }
}
#[derive(Default)]
pub struct Scheduler {
    last: Option<i64>,
    timezone: Option<String>,
}
pub fn in_window(settings: &Settings, now: i64) -> bool {
    let local = settings.local_datetime(now);
    let minute = local.hour() * 60 + local.minute();
    (480..=1080).contains(&minute)
}
pub fn relevant(task: &Task, settings: &Settings, now: i64) -> bool {
    task.archived_at_utc.is_none()
        && !task.snoozed_until_utc.is_some_and(|u| u > now)
        && if let Some(date) = &task.local_date {
            date <= &settings.local_datetime(now).date_naive().to_string()
        } else {
            task.due_at_utc.is_none_or(|due| {
                settings.local_datetime(due).date_naive()
                    <= settings.local_datetime(now).date_naive()
            })
        }
}
pub fn overdue(task: &Task, settings: &Settings, now: i64) -> bool {
    task.active()
        && task.due_at_utc.is_some_and(|due| {
            if task.is_all_day {
                task.local_date
                    .clone()
                    .unwrap_or_else(|| settings.local_datetime(due).date_naive().to_string())
                    < settings.local_datetime(now).date_naive().to_string()
            } else {
                due < now
            }
        })
}
pub fn summary(tasks: &[Task], s: &Settings, now: i64) -> String {
    let tasks: Vec<_> = tasks.iter().filter(|t| relevant(t, s, now)).collect();
    let done = tasks
        .iter()
        .filter(|t| {
            t.status == "done"
                && t.completed_at_utc.is_some_and(|at| {
                    s.local_datetime(at).date_naive() == s.local_datetime(now).date_naive()
                })
        })
        .count();
    let active: Vec<_> = tasks.iter().filter(|t| t.active()).collect();
    let late = active.iter().filter(|t| overdue(t, s, now)).count();
    let names = active
        .iter()
        .take(3)
        .map(|t| t.title.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "Đã xong: {done} · Còn: {} · Quá hạn: {late}\n{names}",
        active.len()
    )
}
impl Scheduler {
    pub fn tick(&mut self, clock: &impl Clock, tasks: &[Task], s: &Settings) -> Vec<Notice> {
        let now = clock.now();
        let local = s.local_datetime(now);
        let previous = self.last.replace(now);
        let zone_changed = self.timezone.as_deref().is_some_and(|z| z != s.timezone);
        self.timezone = Some(s.timezone.clone());
        if !s.onboarding_done
            || !s.notifications
            || (s.paused && s.pause_until_utc.is_none_or(|u| u > now))
            || !in_window(s, now)
        {
            return vec![];
        }
        let gap = previous.is_none_or(|p| now - p > 90 || now < p) || zone_changed;
        let date = local.date_naive();
        let slot = local.minute() == 0 && local.second() < 60;
        let mut notices = Vec::new();
        let active = tasks.iter().any(|t| t.active() && relevant(t, s, now));
        let has_today = tasks.iter().any(|t| {
            t.archived_at_utc.is_none()
                && (t.local_day(s)==Some(date)
                    || t.created_at_utc > 0
                        && s.local_datetime(t.created_at_utc).date_naive() == date
                    || t.completed_at_utc
                        .is_some_and(|d| s.local_datetime(d).date_naive() == date))
        });
        if slot
            && ((local.hour() == 18 && s.daily_summary && has_today)
                || ((8..18).contains(&local.hour()) && s.hourly && active))
        {
            let kind = if local.hour() == 18 {
                "summary"
            } else if local.hour() == 8 {
                "morning"
            } else {
                "hourly"
            };
            notices.push(Notice {
                key: format!("slot:{date}:{}", local.hour()),
                kind: kind.into(),
                title: match kind {
                    "summary" => "Tổng kết ngày",
                    "morning" => "Bắt đầu ngày mới",
                    _ => "Nhắc việc đầu giờ",
                }
                .into(),
                body: summary(tasks, s, now),
            });
        } else if gap && active {
            notices.push(Notice {
                key: format!("reconcile:{date}:{}", local.hour()),
                kind: "reconcile".into(),
                title: "Công việc đang chờ".into(),
                body: summary(tasks, s, now),
            });
        }
        // After wake, old deadlines are represented in ONE summary. No old events replayed.
        if gap {
            return notices;
        }
        let previous = previous.unwrap_or(now).max(now - 60);
        for task in tasks.iter().filter(|t| t.active() && !t.is_all_day) {
            let Some(due) = task.effective_due() else {
                continue;
            };
            for (kind, event, enabled) in [
                ("due", due, s.due_alert),
                ("pre", due - s.pre_minutes * 60, s.pre_alert),
            ] {
                if enabled
                    && event > previous
                    && event <= now
                    && event >= task.updated_at_utc
                    && in_window(s, event)
                    && !task.snoozed_until_utc.is_some_and(|u| event < u && now < u)
                {
                    notices.push(Notice {
                        key: format!("task:{}:{due}:{kind}", task.id),
                        kind: kind.into(),
                        title: if kind == "due" {
                            "Đến hạn"
                        } else {
                            "Sắp đến hạn"
                        }
                        .into(),
                        body: format!(
                            "{} · {:02}:{:02}",
                            task.title,
                            s.local_datetime(due).hour(),
                            s.local_datetime(due).minute()
                        ),
                    });
                }
            }
        }
        notices.sort_by_key(|n| match n.kind.as_str() {
            "due" => 0,
            "pre" => 1,
            _ => 2,
        });
        notices
    }
}
pub fn aggregate(notices: &[Notice]) -> (String, String) {
    let title = if notices.len() == 1 {
        notices[0].title.clone()
    } else {
        "Task Note AI · Nhắc việc".into()
    };
    let body = notices
        .iter()
        .take(5)
        .map(|n| format!("{}: {}", n.title, n.body))
        .collect::<Vec<_>>()
        .join("\n");
    let body = body.chars().take(480).collect();
    (title, body)
}
pub fn next_slot(s: &Settings, now: i64) -> String {
    let local = s.local_datetime(now);
    let hour = local.hour();
    let next = if hour < 8 {
        8
    } else if hour < 18 {
        hour + 1
    } else {
        8
    };
    format!(
        "{} {:02}:00",
        if hour >= 18 { "Ngày mai" } else { "Hôm nay" },
        next
    )
}
