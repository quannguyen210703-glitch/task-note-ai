use chrono::{NaiveDate, TimeZone};
use task_note_core::{
    actions::Action,
    chat_v2::offline_inputs,
    cloud::{AiReply, AiTask},
    goals::Goal,
    scheduler::{overdue, Clock, Scheduler},
    service::{Command, Request},
    *,
};
fn at(day: &str, h: u32, m: u32) -> i64 {
    chrono_tz::Asia::Ho_Chi_Minh
        .from_local_datetime(
            &NaiveDate::parse_from_str(day, "%Y-%m-%d")
                .unwrap()
                .and_hms_opt(h, m, 0)
                .unwrap(),
        )
        .unwrap()
        .timestamp()
}
fn now() -> i64 {
    at("2026-10-14", 10, 0)
}
#[tokio::test]
async fn goal_batch_keeps_parent_dependencies_estimates_and_undo_guard() {
    let r = Repository::memory().await.unwrap();
    let goal = Goal {
        id: uuid::Uuid::new_v4().to_string(),
        title: "Mục tiêu batch".into(),
        description: "Giữ nguyên dữ liệu".into(),
        desired_outcome: String::new(),
        start_date: "2026-10-14".into(),
        deadline: None,
        allowed_days: vec![2],
        auto_add: false,
        status: "active".into(),
        missing_information: String::new(),
    };
    let steps: Vec<_> = (0..3)
        .map(|i| AiTask {
            title: format!("Bước {i}"),
            description: String::new(),
            date: "2026-10-14".into(),
            time: None,
            priority: "medium".into(),
            estimate_minutes: Some(20),
            depends_on: if i > 0 { Some(i - 1) } else { None },
        })
        .collect();
    let result = r
        .mutate(
            &Action::GoalBatch {
                goal: goal.clone(),
                steps,
                original: "Chia và thêm".into(),
            },
            "goal-batch",
            now(),
            false,
        )
        .await
        .unwrap();
    assert_eq!(result.tasks.len(), 3);
    assert_eq!(r.goals().await.unwrap().len(), 1);
    let all = r.list(false).await.unwrap();
    assert_eq!(all.iter().filter(|t| t.blocked).count(), 2);
    let step = r.steps(&goal.id).await.unwrap().remove(0);
    r.set_step_estimate(&step.id, Some(45), now() + 1)
        .await
        .unwrap();
    assert_eq!(
        r.get(step.task_id.as_ref().unwrap())
            .await
            .unwrap()
            .estimate_minutes,
        Some(45)
    );
    assert!(r
        .mutate(
            &Action::Undo {
                request_id: "goal-batch".into()
            },
            "unsafe-undo",
            now() + 2,
            false
        )
        .await
        .is_err());
    let task = r.get(step.task_id.as_ref().unwrap()).await.unwrap();
    let mut input = task.input();
    input.status = "done".into();
    r.mutate(
        &Action::Update {
            id: task.id,
            input,
            expected: task.updated_at_utc,
            scope: "one".into(),
        },
        "goal-done",
        now() + 3,
        false,
    )
    .await
    .unwrap();
    assert_eq!(
        r.list(false)
            .await
            .unwrap()
            .iter()
            .filter(|t| t.blocked)
            .count(),
        1
    );
}
struct Fake(i64);
#[tokio::test]
async fn all_day_cli_date_survives_timezone_change(){
    let r=Repository::memory().await.unwrap();let mut input=TaskInput::new("Ngày local",Some(at("2026-10-14",0,0)));input.is_all_day=true;input.recurrence_rule=Some("daily".into());r.create(&input,now()).await.unwrap();
    let mut s=r.settings().await.unwrap();s.timezone="UTC".into();r.save_settings(&s).await.unwrap();
    let current=chrono::Utc::now().timestamp();let dir=tempfile::tempdir().unwrap();
    let request=Request{request_id:"local-date".into(),received_at_utc:current,expires_at_utc:current+60,dry_run:false,action:Command::List{today:false,date:Some("2026-10-14".into()),status:None,trash:false}};
    let reply=r.execute(&request,dir.path(),false).await;assert!(reply.ok);assert_eq!(reply.data.unwrap().as_array().unwrap().len(),1);
}
#[test]
fn stale_or_partial_v1_status_never_allows_second_writer(){
    let dir=tempfile::tempdir().unwrap();let path=dir.path().join("engine-status.json");
    std::fs::write(&path,r#"{"last_check":1,"running":true}"#).unwrap();assert!(task_note_core::coordinator::legacy_active(dir.path()));
    std::fs::write(&path,"{").unwrap();assert!(task_note_core::coordinator::legacy_active(dir.path()));
    std::fs::write(&path,r#"{"version":"2.0.0","last_check":1}"#).unwrap();assert!(!task_note_core::coordinator::legacy_active(dir.path()));
}
impl Clock for Fake {
    fn now(&self) -> i64 {
        self.0
    }
}
fn add(title: &str) -> Action {
    Action::Add {
        inputs: vec![TaskInput::new(title, Some(at("2026-10-14", 16, 0)))],
        source: "chat".into(),
        original: "Thêm việc".into(),
    }
}
#[test]
fn offline_dates_all_day_and_intent() {
    let s = Settings::default();
    let trials=offline_inputs("Thêm việc thử đồng bộ, thu tiền, gửi email",&s,now(),None).unwrap().unwrap();
    assert_eq!(trials.len(),3);assert!(trials.iter().all(|t|t.due_at_utc==Some(at("2026-10-14",0,0))));
    let named=offline_inputs("Thêm việc gọi chị Mai",&s,now(),None).unwrap().unwrap();
    assert_eq!(named[0].title,"gọi chị Mai");assert_eq!(named[0].due_at_utc,Some(at("2026-10-14",0,0)));
    let draft_task=offline_inputs("Thêm việc soạn checklist",&s,now(),None).unwrap().unwrap();assert_eq!(draft_task[0].title,"soạn checklist");
    assert!(offline_inputs("Thêm việc đối chiếu dữ liệu",&s,now(),None).unwrap().unwrap()[0].description.is_empty());
    let recurring=offline_inputs("Nhắc anh họp nhóm mỗi thứ Hai lúc 9h",&s,now(),None).unwrap().unwrap();assert_eq!(recurring[0].title,"họp nhóm");assert_eq!(recurring[0].recurrence_rule.as_deref(),Some("weekly:0"));
    let tasks = offline_inputs(
        "Hôm nay kiểm tra logo, 16h gọi khách, ngày mai gửi báo cáo",
        &s,
        now(),
        Some("2026-11-01"),
    )
    .unwrap()
    .unwrap();
    assert_eq!(tasks.len(), 3);
    assert_eq!(tasks[0].title, "kiểm tra logo");
    assert!(tasks[0].is_all_day);
    assert_eq!(tasks[1].due_at_utc, Some(at("2026-10-14", 16, 0)));
    assert_eq!(tasks[2].due_at_utc, Some(at("2026-10-15", 0, 0)));
    let afternoon = offline_inputs("Chiều mai kiểm tra bố cục", &s, now(), None)
        .unwrap()
        .unwrap();
    assert!(afternoon[0].is_all_day);
    assert!(!afternoon[0].description.is_empty());
    for text in [
        "Anh đang cân nhắc có nên sửa cẩm nang không",
        "Em viết nháp checklist giúp anh",
        "\"Mai 9h gọi khách\"",
        "Gợi ý việc cần làm",
    ] {
        assert!(offline_inputs(text, &s, now(), None).unwrap().is_none());
    }
    assert!(offline_inputs("Thêm việc họp vào thứ Sáu", &s, now(), None).is_err());
    assert!(offline_inputs("Mai 25h gọi khách", &s, now(), None).is_err());
}
#[tokio::test]
async fn auto_chat_atomic_retry_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("task.sqlite");
    let r = Repository::open(&path).await.unwrap();
    let a = r
        .chat_v2(
            "Hôm nay kiểm tra logo, 16h gọi khách, ngày mai gửi báo cáo",
            "request-a",
            now(),
            Some("2026-11-01"),
            false,
        )
        .await
        .unwrap();
    assert_eq!(a.saved.len(), 3);
    assert!(a.text.starts_with("Đã thêm 3"));
    assert!(a.proposals.is_empty());
    let b = r
        .chat_v2(
            "Hôm nay kiểm tra logo, 16h gọi khách, ngày mai gửi báo cáo",
            "request-a",
            now() + 86400,
            None,
            false,
        )
        .await
        .unwrap();
    assert_eq!(a.saved[0].id, b.saved[0].id);
    r.pool.close().await;
    let r = Repository::open(&path).await.unwrap();
    assert_eq!(r.list(false).await.unwrap().len(), 3);
    let dates = r.list(false).await.unwrap();
    assert_eq!(
        dates
            .iter()
            .find(|t| t.title == "kiểm tra logo")
            .unwrap()
            .local_date
            .as_deref(),
        Some("2026-10-14")
    );
}
#[tokio::test]
async fn ambiguous_batch_continues_without_partial_commit() {
    let r = Repository::memory().await.unwrap();
    let a = r
        .chat_v2(
            "Thêm việc kiểm tra logo, họp vào thứ Sáu",
            "amb",
            now(),
            None,
            false,
        )
        .await
        .unwrap();
    assert!(a.pending);
    assert!(r.list(false).await.unwrap().is_empty());
    let a = r
        .chat_v2("16/10/2026", "amb", now() + 30, None, false)
        .await
        .unwrap();
    assert_eq!(a.saved.len(), 2);
    assert_eq!(r.list(false).await.unwrap().len(), 2);
}
#[tokio::test]
async fn past_and_duplicate_require_whole_batch_preview() {
    let r = Repository::memory().await.unwrap();
    let a = r
        .chat_v2(
            "Hôm nay 9h gọi khách, 16h kiểm tra logo",
            "past",
            now(),
            None,
            false,
        )
        .await
        .unwrap();
    assert!(a.batch_token.is_some());
    assert_eq!(a.saved.len(), 2);
    assert!(r.list(false).await.unwrap().is_empty());
    let out = r
        .confirm_batch(a.batch_token.as_ref().unwrap(), now() + 1)
        .await
        .unwrap();
    assert_eq!(out.tasks.len(), 2);
    r.confirm_batch(a.batch_token.as_ref().unwrap(), now() + 2)
        .await
        .unwrap();
    assert_eq!(r.list(false).await.unwrap().len(), 2);
}
#[tokio::test]
async fn atomic_failure_receipt_and_dry_run() {
    let r = Repository::memory().await.unwrap();
    let bad = Action::Add {
        inputs: vec![TaskInput::new("Đúng", None), TaskInput::new(" ", None)],
        source: "chat".into(),
        original: String::new(),
    };
    assert!(r.mutate(&bad, "bad", now(), false).await.is_err());
    assert!(r.list(false).await.unwrap().is_empty());
    let dry = r.mutate(&add("Báo cáo"), "dry", now(), true).await.unwrap();
    assert_eq!(dry.tasks.len(), 1);
    assert!(r.list(false).await.unwrap().is_empty());
    let first = r
        .mutate(&add("Báo cáo"), "same", now(), false)
        .await
        .unwrap();
    let second = r
        .mutate(&add("Báo cáo"), "same", now() + 60, false)
        .await
        .unwrap();
    assert_eq!(first.tasks[0].id, second.tasks[0].id);
    assert!(r
        .mutate(&add("Việc khác"), "same", now(), false)
        .await
        .is_err());
    assert_eq!(r.list(false).await.unwrap().len(), 1);
}
#[tokio::test]
async fn snooze_unsnooze_delete_restore_and_global_pause() {
    let r = Repository::memory().await.unwrap();
    let mut input = TaskInput::new("Đang làm", Some(now() - 60));
    input.status = "doing".into();
    input.description = "Ghi chú & [ ] checklist".into();
    let task = r.create(&input, now()).await.unwrap();
    r.mutate(
        &Action::Pause {
            until: Some(now() + 3600),
        },
        "pause",
        now(),
        false,
    )
    .await
    .unwrap();
    r.mutate(
        &Action::Snooze {
            id: task.id.clone(),
            until: now() + 1800,
            expected: task.updated_at_utc,
        },
        "snooze",
        now(),
        false,
    )
    .await
    .unwrap();
    assert!(overdue(
        &r.get(&task.id).await.unwrap(),
        &Settings::default(),
        now()
    ));
    let out = r
        .mutate(
            &Action::Unsnooze {
                id: task.id.clone(),
            },
            "unsnooze",
            now() + 1,
            false,
        )
        .await
        .unwrap();
    assert_eq!(out.tasks[0].input().description, input.description);
    assert_eq!(out.tasks[0].status, "doing");
    assert_eq!(out.tasks[0].due_at_utc, input.due_at_utc);
    assert!(r.settings().await.unwrap().paused);
    let again = r
        .mutate(
            &Action::Unsnooze {
                id: task.id.clone(),
            },
            "again",
            now() + 2,
            false,
        )
        .await
        .unwrap();
    assert!(again.message.contains("không còn"));
    let t = r.get(&task.id).await.unwrap();
    r.mutate(
        &Action::Delete {
            id: t.id.clone(),
            expected: t.updated_at_utc,
            scope: "one".into(),
        },
        "delete",
        now() + 3,
        false,
    )
    .await
    .unwrap();
    assert!(r.list(false).await.unwrap().is_empty());
    r.mutate(
        &Action::Unsnooze { id: t.id.clone() },
        "stale-unsnooze",
        now() + 4,
        false,
    )
    .await
    .unwrap();
    assert!(r.get(&t.id).await.unwrap().archived_at_utc.is_some());
    r.mutate(
        &Action::Restore { id: t.id.clone() },
        "restore",
        now() + 5,
        false,
    )
    .await
    .unwrap();
    assert_eq!(
        r.get(&t.id).await.unwrap().input().description,
        input.description
    );
}
#[test]
fn snooze_hourly_suppressed_and_expired_deadline_no_replay() {
    let mut t = task_note_core::repository::task_from_input(
        &TaskInput::new("Việc", Some(at("2026-10-14", 10, 0))),
        now(),
    );
    t.snoozed_until_utc = Some(at("2026-10-14", 11, 30));
    let s = Settings {
        onboarding_done: true,
        ..Settings::default()
    };
    let mut engine = Scheduler::default();
    engine.tick(&Fake(at("2026-10-14", 10, 59)), &[t.clone()], &s);
    assert!(engine
        .tick(&Fake(at("2026-10-14", 11, 0)), &[t.clone()], &s)
        .is_empty());
    engine.tick(&Fake(at("2026-10-14", 11, 29) + 40), &[t.clone()], &s);
    assert_eq!(
        engine
            .tick(&Fake(at("2026-10-14", 11, 30)), &[t.clone()], &s)
            .len(),
        1
    );
    t.snoozed_until_utc = None;
    assert!(engine
        .tick(&Fake(at("2026-10-14", 11, 30) + 20), &[t.clone()], &s)
        .is_empty());
    assert!(engine
        .tick(&Fake(at("2026-10-14", 19, 0)), &[t], &s)
        .is_empty());
}
#[test]
fn unsnooze_after_deadline_inside_tick_does_not_replay_old_deadline(){
    let mut task=task_note_core::repository::task_from_input(&TaskInput::new("Giữ hạn",Some(at("2026-10-14",10,30))),at("2026-10-14",8,0));
    task.snoozed_until_utc=Some(at("2026-10-14",11,30));
    let s=Settings{onboarding_done:true,..Settings::default()};let mut engine=Scheduler::default();
    engine.tick(&Fake(at("2026-10-14",10,29)+40),&[task.clone()],&s);
    task.snoozed_until_utc=None;task.updated_at_utc=at("2026-10-14",10,30)+10;
    assert!(engine.tick(&Fake(at("2026-10-14",10,30)+20),&[task],&s).is_empty());
}
#[test]
fn codex_is_a_valid_local_ai_provider_without_api_model() {
    let mut settings = Settings::default();
    settings.ai_provider = "codex".into();
    settings.ai_model.clear();
    assert!(settings.validate().is_ok());
    settings.ai_provider = "unknown".into();
    assert!(settings.validate().is_err());
}

#[tokio::test]
async fn undo_refuses_edited_batch() {
    let r = Repository::memory().await.unwrap();
    let task = r
        .mutate(&add("Báo cáo"), "batch", now(), false)
        .await
        .unwrap()
        .tasks
        .remove(0);
    let mut input = task.input();
    input.title = "Đã sửa".into();
    r.mutate(
        &Action::Update {
            id: task.id.clone(),
            input,
            expected: task.updated_at_utc,
            scope: "one".into(),
        },
        "edit",
        now() + 1,
        false,
    )
    .await
    .unwrap();
    assert!(r
        .mutate(
            &Action::Undo {
                request_id: "batch".into()
            },
            "undo",
            now() + 2,
            false
        )
        .await
        .is_err());
    assert_eq!(r.list(false).await.unwrap().len(), 1);
}
#[tokio::test]
async fn backup_with_wal_includes_latest_committed_data() {
    let dir = tempfile::tempdir().unwrap();
    let r = Repository::open(&dir.path().join("live.sqlite"))
        .await
        .unwrap();
    let t = r
        .mutate(&add("Unicode tiếng Việt 'quote'"), "wal", now(), false)
        .await
        .unwrap()
        .tasks
        .remove(0);
    let path = dir.path().join("snapshot with spaces.sqlite");
    r.backup_database(&path).await.unwrap();
    let other = Repository::open_readonly(&path).await.unwrap();
    assert_eq!(other.get(&t.id).await.unwrap().title, t.title);
    assert!(
        task_note_core::service::verify_backup(&path).await.unwrap()["verified"]
            .as_bool()
            .unwrap()
    );
    assert!(r.backup_database(&path).await.is_err());
}
#[tokio::test]
async fn owner_lock_exclusive_released_after_drop() {
    let dir = tempfile::tempdir().unwrap();
    let first = task_note_core::coordinator::try_owner(dir.path())
        .unwrap()
        .unwrap();
    assert!(task_note_core::coordinator::try_owner(dir.path())
        .unwrap()
        .is_none());
    drop(first);
    assert!(task_note_core::coordinator::try_owner(dir.path())
        .unwrap()
        .is_some());
}
#[test]
fn provider_schema_rejects_extra_fields_and_bad_dependencies() {
    assert!(serde_json::from_str::<AiReply>(
        r#"{"action":"sql","message":"","tasks":[],"shell":"whoami"}"#
    )
    .is_err());
    let ai = AiReply {
        action: "create_tasks".into(),
        message: String::new(),
        tasks: vec![AiTask {
            title: "A".into(),
            description: String::new(),
            date: "2026-10-14".into(),
            time: None,
            priority: "medium".into(),
            estimate_minutes: Some(20),
            depends_on: Some(0),
        }],
    };
    assert!(task_note_core::cloud::validate_reply(&ai, &Settings::default(), now()).is_err());
}
#[tokio::test]
async fn planner_budget_restart_dependencies_and_tombstones() {
    let r = Repository::memory().await.unwrap();
    let mut s = Settings::default();
    s.auto_plan = true;
    r.save_settings(&s).await.unwrap();
    let goal = Goal {
        id: uuid::Uuid::new_v4().to_string(),
        title: "Mục tiêu thử".into(),
        description: "Giữ nguyên nội dung".into(),
        desired_outcome: "PDF".into(),
        start_date: "2026-10-14".into(),
        deadline: None,
        allowed_days: vec![2],
        auto_add: true,
        status: "active".into(),
        missing_information: String::new(),
    };
    r.save_goal(&goal, now()).await.unwrap();
    let steps: Vec<_> = (0..5)
        .map(|i| AiTask {
            title: format!("Bước {i}"),
            description: String::new(),
            date: "2026-10-14".into(),
            time: None,
            priority: "medium".into(),
            estimate_minutes: Some(15),
            depends_on: None,
        })
        .collect();
    r.store_steps(&goal.id, &steps).await.unwrap();
    assert!(r.auto_plan_once(now()).await.unwrap());
    assert_eq!(r.list(false).await.unwrap().len(), 3);
    assert!(!r.auto_plan_once(now() + 600).await.unwrap());
    let t = r.list(false).await.unwrap().remove(0);
    r.mutate(
        &Action::Delete {
            id: t.id,
            expected: t.updated_at_utc,
            scope: "one".into(),
        },
        "del-step",
        now() + 1,
        false,
    )
    .await
    .unwrap();
    assert!(!r.auto_plan_once(now() + 1200).await.unwrap());
    assert_eq!(r.list(true).await.unwrap().len(), 3);
    assert!(!r.auto_plan_once(at("2026-10-15", 19, 0)).await.unwrap());
}
#[tokio::test]
async fn service_retry_update_keeps_pinned_action_and_doctor_readonly() {
    let dir = tempfile::tempdir().unwrap();
    let r = Repository::open(&dir.path().join("task-note.sqlite"))
        .await
        .unwrap();
    let current = chrono::Utc::now().timestamp();
    let req = Request {
        request_id: "cli-add".into(),
        received_at_utc: current,
        expires_at_utc: current + 15,
        dry_run: false,
        action: Command::Add {
            title: "A".into(),
            date: None,
            time: None,
        },
    };
    let a = r.execute(&req, dir.path(), false).await;
    assert!(a.ok);
    assert!(r.execute(&req, dir.path(), false).await.ok);
    assert_eq!(r.list(false).await.unwrap().len(), 1);
    let stat = Request {
        request_id: "doctor".into(),
        action: Command::Status,
        ..req
    };
    assert!(r.execute(&stat, dir.path(), false).await.ok);
}
