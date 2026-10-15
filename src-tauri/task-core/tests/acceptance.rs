use chrono::{TimeZone, Utc};
use task_note_core::{
    assistant::parse_time,
    repository::task_from_input,
    scheduler::{overdue, Clock, Scheduler},
    *,
};
struct Fake(i64);
impl Clock for Fake {
    fn now(&self) -> i64 {
        self.0
    }
}
fn at(date: &str, h: u32, m: u32) -> i64 {
    let day = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
    chrono_tz::Asia::Ho_Chi_Minh
        .from_local_datetime(&day.and_hms_opt(h, m, 0).unwrap())
        .unwrap()
        .timestamp()
}
fn t(h: u32, m: u32) -> i64 {
    at("2026-10-14", h, m)
}
fn settings() -> Settings {
    Settings {
        onboarding_done: true,
        ..Settings::default()
    }
}
fn task() -> Task {
    task_from_input(
        &TaskInput::new("Kiểm tra báo cáo", Some(t(10, 30))),
        t(7, 0),
    )
}
#[test]
fn t07_t09_hourly_boundaries() {
    let mut scheduler = Scheduler::default();
    let tasks = vec![task()];
    let s = settings();
    assert!(scheduler.tick(&Fake(t(7, 59)), &tasks, &s).is_empty());
    assert_eq!(
        scheduler.tick(&Fake(t(8, 0)), &tasks, &s)[0].kind,
        "morning"
    );
    assert!(scheduler.tick(&Fake(t(8, 1)), &tasks, &s).is_empty());
    for h in 9..18 {
        let n = scheduler.tick(&Fake(t(h, 0)), &tasks, &s);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].kind, "hourly");
    }
    assert_eq!(
        scheduler.tick(&Fake(t(18, 0)), &tasks, &s)[0].kind,
        "summary"
    );
    assert!(scheduler.tick(&Fake(t(18, 1)), &tasks, &s).is_empty());
}
#[test]
fn t10_deadlines_t11_completion_t12_snooze() {
    let mut scheduler = Scheduler::default();
    let mut tasks = vec![task()];
    let s = settings();
    scheduler.tick(&Fake(t(10, 14) + 40), &tasks, &s);
    assert_eq!(scheduler.tick(&Fake(t(10, 15)), &tasks, &s)[0].kind, "pre");
    scheduler.tick(&Fake(t(10, 29) + 40), &tasks, &s);
    assert_eq!(scheduler.tick(&Fake(t(10, 30)), &tasks, &s)[0].kind, "due");
    tasks[0].status = "done".into();
    let mut scheduler = Scheduler::default();
    scheduler.tick(&Fake(t(10, 14) + 40), &tasks, &s);
    assert!(scheduler.tick(&Fake(t(10, 15)), &tasks, &s).is_empty());
    tasks[0].status = "todo".into();
    tasks[0].snoozed_until_utc = Some(t(11, 30));
    let mut scheduler = Scheduler::default();
    scheduler.tick(&Fake(t(10, 14) + 40), &tasks, &s);
    assert!(scheduler.tick(&Fake(t(10, 15)), &tasks, &s).is_empty());
    scheduler.tick(&Fake(t(11, 14) + 40), &tasks, &s);
    // V2: snooze suppresses ALL reminders until its end, including pre-alerts.
    assert!(scheduler.tick(&Fake(t(11, 15)), &tasks, &s).is_empty());
    scheduler.tick(&Fake(t(11, 29) + 40), &tasks, &s);
    assert_eq!(scheduler.tick(&Fake(t(11, 30)), &tasks, &s)[0].kind, "due");
}
#[test]
fn t13_t14_t18_t26_t27_wake_quiet_clock() {
    let mut scheduler = Scheduler::default();
    let tasks = vec![task()];
    let s = settings();
    scheduler.tick(&Fake(t(9, 30)), &tasks, &s);
    let resume = scheduler.tick(&Fake(t(12, 20)), &tasks, &s);
    assert_eq!(resume.len(), 1);
    assert_eq!(resume[0].kind, "reconcile");
    assert!(scheduler.tick(&Fake(t(12, 20) + 20), &tasks, &s).is_empty());
    assert!(scheduler.tick(&Fake(t(19, 0)), &tasks, &s).is_empty());
    assert!(Scheduler::default()
        .tick(&Fake(t(8, 0)), &[], &s)
        .is_empty());
    let muted = Settings {
        paused: true,
        ..s.clone()
    };
    assert!(Scheduler::default()
        .tick(&Fake(t(8, 0)), &tasks, &muted)
        .is_empty());
    let disabled = Settings {
        notifications: false,
        ..s.clone()
    };
    assert!(Scheduler::default()
        .tick(&Fake(t(8, 0)), &tasks, &disabled)
        .is_empty());
    let mut scheduler = Scheduler::default();
    let first = scheduler.tick(&Fake(t(10, 6)), &tasks, &s);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].kind, "reconcile");
    let backward = scheduler.tick(&Fake(t(9, 6)), &tasks, &s);
    assert_eq!(backward.len(), 1);
    assert_eq!(backward[0].kind, "reconcile");
}
#[test]
fn t16_overdue_no_deadline_change_and_allday() {
    let mut task = task();
    let s = settings();
    let due = task.due_at_utc;
    assert!(overdue(&task, &s, at("2026-10-15", 8, 0)));
    assert_eq!(task.due_at_utc, due);
    task.is_all_day = true;
    assert!(!overdue(&task, &s, t(18, 0)));
    assert!(overdue(&task, &s, at("2026-10-15", 8, 0)));
}
#[tokio::test]
async fn t02_t15_persistence_and_dedupe() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tasks.sqlite");
    let repo = Repository::open(&path).await.unwrap();
    let task = repo
        .create(
            &TaskInput::new(
                "Tiếng Việt '); DROP TABLE tasks;-- <script>",
                Some(t(10, 30)),
            ),
            t(8, 0),
        )
        .await
        .unwrap();
    let keys = vec![("slot:2026-10-14:8".into(), "morning".into())];
    assert_eq!(repo.claim(&keys, t(8, 0)).await.unwrap().len(), 1);
    repo.pool.close().await;
    let repo = Repository::open(&path).await.unwrap();
    assert_eq!(repo.get(&task.id).await.unwrap().title, task.title);
    assert_eq!(repo.list(false).await.unwrap().len(), 1);
    assert!(repo.claim(&keys, t(8, 0)).await.unwrap().is_empty());
}
#[tokio::test]
async fn t17_recurrence_and_t22_backup() {
    let repo = Repository::memory().await.unwrap();
    let mut input = TaskInput::new("Họp nhóm", Some(at("2026-10-19", 9, 0)));
    input.recurrence_rule = Some("weekly:0".into());
    let first = repo.create(&input, t(8, 0)).await.unwrap();
    repo.materialize(t(8, 0)).await.unwrap();
    let tasks = repo.list(false).await.unwrap();
    assert_eq!(tasks.len(), 2);
    let mut done = first.input();
    done.status = "done".into();
    repo.update(&first.id, &done, first.updated_at_utc, "one", t(8, 1))
        .await
        .unwrap();
    repo.materialize(t(8, 2)).await.unwrap();
    let tasks = repo.list(false).await.unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks.iter().filter(|t| t.status == "done").count(), 1);
    let backup = serde_json::to_string(&repo.export(t(8, 2)).await.unwrap()).unwrap();
    let other = Repository::memory().await.unwrap();
    assert_eq!(other.preview_import(&backup).await.unwrap().new_tasks, 2);
    other.import(&backup).await.unwrap();
    assert_eq!(
        repo.list(true).await.unwrap(),
        other.list(true).await.unwrap()
    );
    assert_eq!(other.import(&backup).await.unwrap().new_tasks, 0);
    assert!(other
        .import("{\"version\":99,\"tasks\":[],\"templates\":[],\"exported_at_utc\":0}")
        .await
        .is_err());
}
#[tokio::test]
async fn t19_t20_t21_chat_confirm_only_and_stale() {
    let repo = Repository::memory().await.unwrap();
    let reply = repo.chat("Mai 9h gọi khách hàng", t(8, 0)).await.unwrap();
    assert_eq!(reply.proposals.len(), 1);
    assert_eq!(reply.proposals[0].input.title, "gọi khách hàng");
    assert_eq!(
        reply.proposals[0].input.due_at_utc,
        Some(at("2026-10-15", 9, 0))
    );
    assert!(repo.list(false).await.unwrap().is_empty());
    let token = &reply.proposals[0].token;
    repo.confirm(token, t(8, 1)).await.unwrap();
    assert_eq!(repo.list(false).await.unwrap().len(), 1);
    assert!(repo.confirm(token, t(8, 1)).await.is_err());
    repo.create(
        &TaskInput::new("Báo cáo doanh thu", Some(t(14, 0))),
        t(8, 0),
    )
    .await
    .unwrap();
    repo.create(&TaskInput::new("Báo cáo chi phí", Some(t(15, 0))), t(8, 0))
        .await
        .unwrap();
    let reply = repo.chat("Đã xong báo cáo", t(8, 0)).await.unwrap();
    assert_eq!(reply.proposals.len(), 2);
    assert_eq!(
        repo.list(false)
            .await
            .unwrap()
            .iter()
            .filter(|t| t.status == "done")
            .count(),
        0
    );
    repo.confirm(&reply.proposals[0].token, t(8, 1))
        .await
        .unwrap();
    assert_eq!(
        repo.list(false)
            .await
            .unwrap()
            .iter()
            .filter(|t| t.status == "done")
            .count(),
        1
    );
    let weekly = repo
        .chat("Nhắc anh họp nhóm mỗi thứ Hai lúc 9h", t(8, 0))
        .await
        .unwrap();
    assert_eq!(weekly.proposals[0].input.title, "họp nhóm");
    assert_eq!(
        weekly.proposals[0].input.recurrence_rule.as_deref(),
        Some("weekly:0")
    );
    let ambiguous = repo
        .chat("Thêm việc báo cáo thứ Sáu 9h", t(8, 0))
        .await
        .unwrap();
    assert!(ambiguous.proposals.is_empty());
    let all = repo.chat("Hôm nay anh có việc gì?", t(8, 0)).await.unwrap();
    assert!(all.text.contains("Báo cáo"));
    let count = repo
        .chat("Còn bao nhiêu việc chưa làm?", t(8, 0))
        .await
        .unwrap();
    assert!(count.text.contains("2 việc"));
}
#[tokio::test]
async fn transaction_conflict_archive_snooze_and_expiry() {
    let repo = Repository::memory().await.unwrap();
    let task = repo
        .create(&TaskInput::new("Việc", Some(t(10, 30))), t(8, 0))
        .await
        .unwrap();
    repo.snooze(&task.id, t(11, 30), task.updated_at_utc, t(8, 0))
        .await
        .unwrap();
    assert!(repo
        .update(&task.id, &task.input(), task.updated_at_utc, "one", t(8, 0))
        .await
        .is_err());
    let task = repo.get(&task.id).await.unwrap();
    repo.archive(&task.id, "one", task.updated_at_utc, t(8, 1))
        .await
        .unwrap();
    assert!(repo.list(false).await.unwrap().is_empty());
    repo.restore(&task.id, t(8, 2)).await.unwrap();
    assert_eq!(repo.list(false).await.unwrap().len(), 1);
    let reply = repo.chat("Mai 9h gọi khách", t(8, 0)).await.unwrap();
    assert!(repo
        .confirm(&reply.proposals[0].token, t(8, 11))
        .await
        .is_err());
}
#[test]
fn timezone_and_invalid_inputs() {
    let s = settings();
    assert_eq!(
        parse_time("Mai 9h gọi khách", &s, t(8, 0))
            .unwrap()
            .unwrap()
            .0,
        at("2026-10-15", 9, 0)
    );
    assert!(parse_time("Mai 25h báo cáo", &s, t(8, 0)).is_err());
    let newyork = Settings {
        timezone: "America/New_York".into(),
        ..s.clone()
    };
    let timestamp = Utc
        .with_ymd_and_hms(2026, 10, 14, 13, 0, 0)
        .unwrap()
        .timestamp();
    assert_eq!(newyork.local_datetime(timestamp).hour(), 9);
    assert!(TaskInput::new(" ", None).validate().is_err());
}
use chrono::Timelike;

#[tokio::test]
async fn series_edit_does_not_duplicate_moved_occurrence() {
    let repo = Repository::memory().await.unwrap();
    let mut input = TaskInput::new("Họp", Some(at("2026-10-19", 9, 0)));
    input.recurrence_rule = Some("weekly:0".into());
    let first = repo.create(&input, t(8, 0)).await.unwrap();
    input.due_at_utc = Some(at("2026-10-26", 10, 0));
    repo.update(&first.id, &input, first.updated_at_utc, "series", t(8, 1))
        .await
        .unwrap();
    repo.materialize(t(8, 2)).await.unwrap();
    let tasks = repo.list(false).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].due_at_utc, input.due_at_utc);
}

#[tokio::test]
async fn series_edit_preserves_trash() {
    let repo = Repository::memory().await.unwrap();
    let mut input = TaskInput::new("Họp", Some(at("2026-10-19", 9, 0)));
    input.recurrence_rule = Some("weekly:0".into());
    let first = repo.create(&input, t(8, 0)).await.unwrap();
    let next = repo
        .list(false)
        .await
        .unwrap()
        .into_iter()
        .find(|task| task.id != first.id)
        .unwrap();
    repo.archive(&next.id, "one", next.updated_at_utc, t(8, 1))
        .await
        .unwrap();
    input.title = "Họp đã sửa".into();
    repo.update(&first.id, &input, first.updated_at_utc, "series", t(8, 2))
        .await
        .unwrap();
    let all = repo.list(true).await.unwrap();
    assert_eq!(all.len(), 2);
    assert!(repo.get(&next.id).await.unwrap().archived_at_utc.is_some());
    assert_eq!(repo.list(false).await.unwrap().len(), 1);
}
