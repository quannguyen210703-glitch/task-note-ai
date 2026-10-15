use serde::Serialize;
use std::sync::Mutex;
use task_note_core::{actions::Action, coordinator};
use task_note_core::{Repository, Settings, Task, TaskInput};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
mod notifications;

#[derive(Clone, Serialize, Default)]
struct EngineStatus {
    version: String,
    running: bool,
    last_check: Option<i64>,
    next_slot: String,
    permission: String,
    last_error: Option<String>,
    db_path: String,
}
struct AppState {
    repo: Repository,
    status: Mutex<EngineStatus>,
    _owner: coordinator::Owner,
    root: std::path::PathBuf,
    test_profile: bool,
}
type CommandResult<T> = Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
fn uuid_key() -> String {
    uuid::Uuid::new_v4().to_string()
}
#[tauri::command]
async fn task_origin(
    state: tauri::State<'_, AppState>,
    id: String,
) -> CommandResult<Option<serde_json::Value>> {
    state.repo.origin(&id).await.map_err(err)
}
#[tauri::command]
async fn set_step_estimate(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    step_id: String,
    minutes: Option<i64>,
) -> CommandResult<()> {
    state
        .repo
        .set_step_estimate(&step_id, minutes, now())
        .await
        .map_err(err)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn unsnooze_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> CommandResult<String> {
    let output = state
        .repo
        .mutate(&Action::Unsnooze { id }, &uuid_key(), now(), false)
        .await
        .map_err(err)?;
    changed(&app);
    Ok(output.message)
}
#[tauri::command]
async fn confirm_batch(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    token: String,
) -> CommandResult<task_note_core::actions::Outcome> {
    let result = state.repo.confirm_batch(&token, now()).await.map_err(err)?;
    changed(&app);
    Ok(result)
}
#[tauri::command]
async fn undo_batch(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request_id: String,
) -> CommandResult<String> {
    let result = state
        .repo
        .mutate(&Action::Undo { request_id }, &uuid_key(), now(), false)
        .await
        .map_err(err)?;
    changed(&app);
    Ok(result.message)
}
#[tauri::command]
async fn list_goals(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<task_note_core::goals::Goal>> {
    state.repo.goals().await.map_err(err)
}
#[tauri::command]
async fn save_goal(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    goal: task_note_core::goals::Goal,
) -> CommandResult<()> {
    state.repo.save_goal(&goal, now()).await.map_err(err)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn goal_steps(
    state: tauri::State<'_, AppState>,
    goal_id: String,
    generate: bool,
) -> CommandResult<Vec<task_note_core::goals::Step>> {
    if generate {
        state
            .repo
            .generate_steps(&goal_id, now())
            .await
            .map_err(err)
    } else {
        state.repo.steps(&goal_id).await.map_err(err)
    }
}
#[tauri::command]
async fn dismiss_step(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> CommandResult<()> {
    state.repo.dismiss_step(&id).await.map_err(err)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn add_goal_step(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    goal_id: String,
    step_id: String,
    date: String,
) -> CommandResult<Task> {
    let s = state.repo.settings().await.map_err(err)?;
    let date = task_note_core::service::date_value(&date, &s, now()).map_err(err)?;
    let task = state
        .repo
        .add_goal_step(&goal_id, &step_id, date, now())
        .await
        .map_err(err)?;
    changed(&app);
    Ok(task)
}
#[tauri::command]
async fn replan(state: tauri::State<'_, AppState>) -> CommandResult<serde_json::Value> {
    let s = state.repo.settings().await.map_err(err)?;
    state
        .repo
        .plan(s.local_datetime(now()).date_naive(), now(), true)
        .await
        .map_err(err)
}
#[tauri::command]
fn ai_key(action: String, key: Option<String>) -> CommandResult<bool> {
    match action.as_str() {
        "save" => {
            task_note_core::cloud::save_key(key.as_deref().ok_or("Thiếu khóa")?).map_err(err)?
        }
        "delete" => task_note_core::cloud::delete_key().map_err(err)?,
        "status" => {}
        _ => return Err("Thao tác không hợp lệ".into()),
    }
    Ok(task_note_core::cloud::has_key())
}
#[tauri::command]
async fn ai_test(state: tauri::State<'_, AppState>) -> CommandResult<String> {
    task_note_core::cloud::test_connection(&state.repo.settings().await.map_err(err)?)
        .await
        .map_err(err)
}

#[tauri::command]
async fn codex_status() -> task_note_core::cloud::CodexStatus {
    task_note_core::cloud::codex_status().await
}

#[tauri::command]
async fn codex_login() -> CommandResult<task_note_core::cloud::CodexStatus> {
    task_note_core::cloud::codex_login().await.map_err(err)
}
#[tauri::command]
async fn save_draft(
    state: tauri::State<'_, AppState>,
    key: String,
    json: String,
) -> CommandResult<()> {
    if key.len() > 120 || json.len() > 32000 {
        return Err("Draft quá dài".into());
    }
    task_note_core::draft::save(&state.repo, &key, &json)
        .await
        .map_err(err)
}
#[tauri::command]
async fn load_draft(
    state: tauri::State<'_, AppState>,
    key: String,
) -> CommandResult<Option<String>> {
    task_note_core::draft::load(&state.repo, &key)
        .await
        .map_err(err)
}
fn startup_command(path: &std::path::Path) -> String {
    format!("\"{}\"", path.display())
}
fn update_autostart(enabled: bool) -> CommandResult<()> {
    let path = startup_command(&std::env::current_exe().map_err(err)?);
    let launcher = auto_launch::AutoLaunch::new(
        "Task Note AI",
        &path,
        auto_launch::WindowsEnableMode::CurrentUser,
        &Vec::<String>::new(),
    );
    if enabled {
        launcher.enable().map_err(err)
    } else {
        launcher.disable().map_err(err)
    }
}
fn launch_tab(args: &[String]) -> Option<String> {
    for (flag, tab) in [
        ("--settings", "settings"),
        ("--assistant", "assistant"),
        ("--dashboard", "today"),
        ("--add", "add"),
    ] {
        if args.iter().any(|arg| arg == flag) {
            return Some(tab.into());
        }
    }
    None
}
#[tauri::command]
fn initial_tab() -> Option<String> {
    launch_tab(&std::env::args().collect::<Vec<_>>())
}
fn changed(app: &tauri::AppHandle) {
    let _ = app.emit("tasks-changed", ());
}

#[tauri::command]
async fn list_tasks(state: tauri::State<'_, AppState>) -> CommandResult<Vec<Task>> {
    state.repo.list(true).await.map_err(err)
}
#[tauri::command]
async fn create_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    input: TaskInput,
) -> CommandResult<Task> {
    let task = state
        .repo
        .mutate(
            &Action::Add {
                inputs: vec![input],
                source: "manual".into(),
                original: String::new(),
            },
            &uuid_key(),
            now(),
            false,
        )
        .await
        .map_err(err)?
        .tasks
        .remove(0);
    changed(&app);
    Ok(task)
}
#[tauri::command]
async fn update_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    input: TaskInput,
    expected: i64,
    scope: String,
) -> CommandResult<Task> {
    let task = state
        .repo
        .mutate(
            &Action::Update {
                id,
                input,
                expected,
                scope,
            },
            &uuid_key(),
            now(),
            false,
        )
        .await
        .map_err(err)?
        .tasks
        .remove(0);
    changed(&app);
    Ok(task)
}
#[tauri::command]
async fn snooze_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    until: i64,
    expected: i64,
) -> CommandResult<()> {
    state
        .repo
        .mutate(
            &Action::Snooze {
                id,
                until,
                expected,
            },
            &uuid_key(),
            now(),
            false,
        )
        .await
        .map_err(err)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn archive_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    scope: String,
    expected: i64,
) -> CommandResult<()> {
    state
        .repo
        .mutate(
            &Action::Delete {
                id,
                scope,
                expected,
            },
            &uuid_key(),
            now(),
            false,
        )
        .await
        .map_err(err)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn restore_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> CommandResult<()> {
    state
        .repo
        .mutate(&Action::Restore { id }, &uuid_key(), now(), false)
        .await
        .map_err(err)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> CommandResult<Settings> {
    state.repo.settings().await.map_err(err)
}
#[tauri::command]
async fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> CommandResult<()> {
    settings.validate().map_err(err)?;
    let previous = state.repo.settings().await.map_err(err)?;
    if state.test_profile && settings.autostart {
        return Err("Profile thử không được bật autostart.".into());
    }
    if !state.test_profile && settings.autostart {
        update_autostart(true)?;
    } else if !state.test_profile {
        update_autostart(false)?;
    }
    if let Err(e) = state.repo.save_settings(&settings).await {
        if previous.autostart {
            let _ = update_autostart(true);
        } else {
            let _ = update_autostart(false);
        }
        return Err(err(e));
    }
    if let Some(w) = app.get_webview_window("sticky") {
        w.set_always_on_top(settings.pinned).map_err(err)?;
    }
    let _ = app.emit("settings-changed", ());
    Ok(())
}
#[tauri::command]
fn engine_status(state: tauri::State<'_, AppState>) -> EngineStatus {
    state.status.lock().unwrap().clone()
}
#[tauri::command]
async fn test_notification(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> CommandResult<String> {
    if state.test_profile {
        return Err("Thông báo native bị tắt trong profile thử.".into());
    }
    let permission = notifications::permission(&app.config().identifier)?;
    if permission != "Enabled" && permission != "NotInitialized" {
        return Err(format!("Windows đang chặn thông báo ({permission}). Mở Settings → System → Notifications & actions để bật Task Note AI."));
    }
    let s = state.repo.settings().await.map_err(err)?;
    notifications::send(
        &app.config().identifier,
        "Task Note AI · Thông báo thử",
        "Đây là thông báo thử do bạn yêu cầu. Công việc của bạn được lưu trên máy.",
        s.sound,
    )?;
    Ok("Đã gửi yêu cầu đến Windows. Kiểm tra toast hoặc Trung tâm thông báo; Focus Assist có thể ẩn toast.".into())
}
#[tauri::command]
async fn chat(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    text: String,
    request_id: String,
    view_date: Option<String>,
) -> CommandResult<task_note_core::assistant::Reply> {
    let reply = state
        .repo
        .chat_v2(&text, &request_id, now(), view_date.as_deref(), false)
        .await
        .map_err(err)?;
    changed(&app);
    Ok(reply)
}
#[tauri::command]
async fn confirm_chat(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    token: String,
) -> CommandResult<Task> {
    let task = state.repo.confirm(&token, now()).await.map_err(err)?;
    changed(&app);
    Ok(task)
}
#[tauri::command]
async fn chat_history(state: tauri::State<'_, AppState>) -> CommandResult<Vec<(String, String)>> {
    state.repo.chat_history().await.map_err(err)
}
#[tauri::command]
async fn clear_chat(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.repo.clear_chat().await.map_err(err)
}
#[tauri::command]
async fn export_backup(state: tauri::State<'_, AppState>) -> CommandResult<String> {
    serde_json::to_string_pretty(&state.repo.export(now()).await.map_err(err)?).map_err(err)
}
#[tauri::command]
async fn save_backup_file(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> CommandResult<String> {
    let json =
        serde_json::to_string_pretty(&state.repo.export(now()).await.map_err(err)?).map_err(err)?;
    let selection = app
        .dialog()
        .file()
        .add_filter("Task Note AI JSON", &["json"])
        .set_file_name(format!(
            "TaskNoteAI-{}.json",
            chrono::Local::now().format("%Y-%m-%d")
        ))
        .blocking_save_file();
    if let Some(file) = selection {
        let path = file.into_path().map_err(err)?;
        std::fs::write(&path, json).map_err(err)?;
        Ok(format!("Đã sao lưu: {}", path.display()))
    } else {
        Ok("Đã hủy xuất sao lưu.".into())
    }
}
#[tauri::command]
async fn save_sqlite_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> CommandResult<String> {
    let selection = app
        .dialog()
        .file()
        .add_filter("SQLite full backup", &["sqlite"])
        .set_file_name(format!(
            "TaskNoteAI-{}.sqlite",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        ))
        .blocking_save_file();
    if let Some(file) = selection {
        let path = file.into_path().map_err(err)?;
        state.repo.backup_database(&path).await.map_err(err)?;
        task_note_core::service::verify_backup(&path)
            .await
            .map_err(err)?;
        Ok(format!(
            "Đã sao lưu SQLite đầy đủ và kiểm chứng: {}",
            path.display()
        ))
    } else {
        Ok("Đã hủy sao lưu.".into())
    }
}
#[tauri::command]
async fn preview_import(
    state: tauri::State<'_, AppState>,
    json: String,
) -> CommandResult<task_note_core::ImportPreview> {
    state.repo.preview_import(&json).await.map_err(err)
}
#[tauri::command]
async fn import_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    json: String,
) -> CommandResult<task_note_core::ImportPreview> {
    let result = state.repo.import(&json).await.map_err(err)?;
    changed(&app);
    Ok(result)
}
#[tauri::command]
fn open_dashboard(app: tauri::AppHandle, tab: String) -> CommandResult<()> {
    let w = app
        .get_webview_window("dashboard")
        .ok_or("Không tìm thấy Dashboard")?;
    w.show().map_err(err)?;
    w.unminimize().map_err(err)?;
    w.set_focus().map_err(err)?;
    let _ = app.emit_to("dashboard", "navigate", tab);
    Ok(())
}
#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) -> CommandResult<()> {
    window.hide().map_err(err)
}
#[tauri::command]
fn collapse_note(window: tauri::WebviewWindow, collapsed: bool) -> CommandResult<()> {
    window
        .set_min_size(Some(tauri::LogicalSize::new(
            320.0,
            if collapsed { 100.0 } else { 360.0 },
        )))
        .map_err(err)?;
    window
        .set_size(tauri::LogicalSize::new(
            360.0,
            if collapsed { 112.0 } else { 520.0 },
        ))
        .map_err(err)
}
fn fit_note(app: &tauri::AppHandle, initial: bool) -> tauri::Result<()> {
    let Some(w) = app.get_webview_window("sticky") else {
        return Ok(());
    };
    let monitors = w.available_monitors()?;
    let position = w.outer_position()?;
    let size = w.outer_size()?;
    let visible = monitors.iter().any(|m| {
        let a = m.work_area();
        position.x >= a.position.x
            && position.y >= a.position.y
            && position.x + size.width as i32 <= a.position.x + a.size.width as i32
            && position.y + size.height as i32 <= a.position.y + a.size.height as i32
    });
    if initial || !visible {
        if let Some(m) = w.primary_monitor()? {
            let a = m.work_area();
            let scale = m.scale_factor();
            let max_width = (a.size.width as f64 / scale - 24.0).max(320.0);
            let max_height = (a.size.height as f64 / scale - 24.0).max(360.0);
            w.set_size(tauri::LogicalSize::new(
                360.0_f64.min(max_width),
                520.0_f64.min(max_height),
            ))?;
            let size = w.outer_size()?;
            w.set_position(tauri::PhysicalPosition::new(
                a.position.x + (a.size.width as i32 - size.width as i32 - 16).max(0),
                a.position.y + 16,
            ))?;
        }
    }
    Ok(())
}
#[tauri::command]
fn reset_position(app: tauri::AppHandle) -> CommandResult<()> {
    fit_note(&app, true).map_err(err)
}

fn start_engine(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        use task_note_core::scheduler::{aggregate, next_slot, Scheduler, WallClock};
        let mut scheduler = Scheduler::default();
        let mut last_materialize = 0;
        loop {
            let state = app.state::<AppState>();
            let timestamp = now();
            let result: CommandResult<()> = async {
                let settings = state.repo.settings().await.map_err(err)?;
                if !state.test_profile && state.repo.auto_plan_once(timestamp).await.map_err(err)? {
                    changed(&app);
                }
                if timestamp - last_materialize > 60 || timestamp < last_materialize {
                    state.repo.materialize(timestamp).await.map_err(err)?;
                    last_materialize = timestamp;
                    changed(&app);
                }
                let mut tasks = state.repo.list(false).await.map_err(err)?;
                tasks.sort_by_key(|t| {
                    (
                        if task_note_core::scheduler::overdue(t, &settings, timestamp) {
                            0
                        } else {
                            1
                        },
                        match t.priority.as_str() {
                            "high" => 0,
                            "medium" => 1,
                            _ => 2,
                        },
                        t.effective_due().unwrap_or(i64::MAX),
                    )
                });
                let (permission, permission_error) =
                    match notifications::permission(&app.config().identifier) {
                        Ok(permission) => (permission, None),
                        Err(error) => (
                            "Unknown".into(),
                            Some(format!("Không đọc được quyền Windows: {error}")),
                        ),
                    };
                let planned = scheduler.tick(&WallClock, &tasks, &settings);
                if let Some(w) = app.get_webview_window("sticky") {
                    w.set_always_on_top(settings.pinned && !notifications::fullscreen())
                        .map_err(err)?;
                }
                {
                    let mut status = state.status.lock().unwrap();
                    status.running = true;
                    status.last_check = Some(timestamp);
                    status.next_slot = next_slot(&settings, timestamp);
                    status.permission = permission.clone();
                    status.last_error = permission_error;
                }
                if !state.test_profile
                    && (permission == "Enabled" || permission == "NotInitialized")
                    && !planned.is_empty()
                {
                    let _guard = state.repo.mutation_gate.lock().await;
                    let settings = state.repo.settings().await.map_err(err)?;
                    let current_time = now();
                    if !settings.notifications
                        || (settings.paused
                            && settings.pause_until_utc.is_none_or(|u| u > current_time))
                        || !task_note_core::scheduler::in_window(&settings, current_time)
                    {
                        return Ok(());
                    }
                    let current = state.repo.list(false).await.map_err(err)?;
                    let planned: Vec<_> = planned
                        .into_iter()
                        .filter_map(|mut notice| {
                            if notice.key.starts_with("task:") {
                                let matches = current.iter().any(|t| {
                                    t.active()
                                        && !t.snoozed_until_utc.is_some_and(|u| u > current_time)
                                        && t.effective_due().is_some_and(|due| {
                                            notice.key
                                                == format!("task:{}:{due}:{}", t.id, notice.kind)
                                        })
                                });
                                if !matches {
                                    return None;
                                }
                            } else {
                                if notice.kind != "summary"
                                    && !current.iter().any(|t| {
                                        t.active()
                                            && task_note_core::scheduler::relevant(
                                                t,
                                                &settings,
                                                current_time,
                                            )
                                    })
                                {
                                    return None;
                                }
                                notice.body = task_note_core::scheduler::summary(
                                    &current,
                                    &settings,
                                    current_time,
                                );
                            }
                            Some(notice)
                        })
                        .collect();
                    let keys = planned
                        .iter()
                        .map(|n| (n.key.clone(), n.kind.clone()))
                        .collect::<Vec<_>>();
                    let claimed = state.repo.claim(&keys, timestamp).await.map_err(err)?;
                    let notices = planned
                        .into_iter()
                        .filter(|n| claimed.contains(&n.key))
                        .collect::<Vec<_>>();
                    if !notices.is_empty() {
                        let (title, body) = aggregate(&notices);
                        if let Err(e) = notifications::send(
                            &app.config().identifier,
                            &title,
                            &body,
                            settings.sound,
                        ) {
                            state
                                .repo
                                .dispatch_failure(&claimed, &e)
                                .await
                                .map_err(err)?;
                            return Err(e);
                        }
                    }
                }
                Ok(())
            }
            .await;
            if let Err(e) = result {
                state.status.lock().unwrap().last_error = Some(e);
            }
            let _ = app.emit("engine-changed", ());
            {
                let dir = &state.root;
                if let Ok(json) = serde_json::to_string_pretty(&*state.status.lock().unwrap()) {
                    let _ = std::fs::write(dir.join("engine-status.json"), json);
                }
            }
            let _ = fit_note(&app, false);
            tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        }
    });
}
pub fn run() {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };
    let args: Vec<String> = std::env::args().collect();
    let test_root = args
        .iter()
        .position(|a| a == "--test-profile")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from);
    if args.iter().any(|a| a == "--test-profile") && test_root.is_none() {
        notifications::startup_error("Thiếu thư mục sau --test-profile; không mở dữ liệu thật.");
        return;
    }
    let diagnostic_root=test_root.clone();
    let mut builder = tauri::Builder::default();
    let mut context = tauri::generate_context!();
    if let Some(root) = &test_root {
        let Ok(root) = std::path::absolute(root) else {
            notifications::startup_error("Đường dẫn profile thử không hợp lệ");
            return;
        };
        context.config_mut().identifier = "vn.tasknoteai.test".into();
        context.config_mut().product_name = Some("Task Note AI · TEST".into());
        for window in &mut context.config_mut().app.windows {
            window.data_directory = Some(root.join("webview"));
        }
    }
    if test_root.is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if let Some(tab) = launch_tab(&args) {
                let _ = open_dashboard(app.clone(), tab);
            } else if let Some(w) = app.get_webview_window("sticky") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }));
    }
    let result = builder
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::SIZE,
                )
                .build(),
        )
        .setup(move |app| {
            let test_profile=test_root.is_some();
            let dir = if let Some(root)=&test_root{std::path::absolute(root)?}else{app.path().app_data_dir()?};
            std::fs::create_dir_all(&dir)?;
            if !test_profile && dir!=coordinator::data_root()? {return Err("Data root không khớp profile Windows hiện tại; dừng để bảo toàn dữ liệu.".into());}
            let mut owner=coordinator::try_owner(&dir)?;
            for _ in 0..50{if owner.is_some(){break;}std::thread::sleep(std::time::Duration::from_millis(100));owner=coordinator::try_owner(&dir)?;}
            let owner=owner.ok_or("Core khác đang sở hữu dữ liệu; thử mở lại sau khi lệnh CLI hoàn tất.")?;
            let db = dir.join("task-note.sqlite");
            let repo = tauri::async_runtime::block_on(Repository::open(&db))?;
            let settings = tauri::async_runtime::block_on(repo.settings())?;
            if !test_profile && settings.onboarding_done && settings.autostart {update_autostart(true)?;}
            app.manage(AppState {
                repo,
                _owner:owner,root:dir.clone(),test_profile,
                status: Mutex::new(EngineStatus {
                    version:env!("CARGO_PKG_VERSION").into(),
                    db_path: db.display().to_string(),
                    ..EngineStatus::default()
                }),
            });
            let handle = app.handle().clone();
            std::fs::write(dir.join("gui-session.json"),serde_json::json!({"pid":std::process::id(),"version":env!("CARGO_PKG_VERSION")}).to_string())?;
            let ipc_app=handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let state=ipc_app.state::<AppState>();
                    match coordinator::drain(&state.repo,&state.root).await{
                        Ok(true)=>{changed(&ipc_app);let _=ipc_app.emit("settings-changed",());},
                        Err(e)=>state.status.lock().unwrap().last_error=Some(e.to_string()),_=>{}
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                }
            });
            fit_note(&handle, !dir.join(".window-state.json").exists())?;
            let sticky = app
                .get_webview_window("sticky")
                .ok_or("Thiếu Sticky Note")?;
            sticky.set_always_on_top(settings.pinned)?;
            if !std::env::args().any(|a|a=="--test-headless") && (settings.show_sticky || !settings.onboarding_done) {
                sticky.show()?;
            }
            let mut items = Vec::new();
            for (id, label) in [
                ("note", "Mở Note"),
                ("dashboard", "Mở bảng quản lý"),
                ("add", "Thêm việc nhanh"),
                ("pause", "Tạm tắt / bật lại thông báo"),
                ("settings", "Cài đặt"),
                ("quit", "Thoát Task Note AI"),
            ] {
                items.push(MenuItem::with_id(app, id, label, true, None::<&str>)?);
            }
            let references: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = items
                .iter()
                .map(|item| item as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
                .collect();
            let menu = Menu::with_items(app, &references)?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().ok_or("Thiếu biểu tượng")?.clone())
                .tooltip("Task Note AI")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        if let Some(w) = tray.app_handle().get_webview_window("sticky") {
                            let _ = w.show();
                        }
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "note" => {
                        let _ = fit_note(app, false);
                        if let Some(w) = app.get_webview_window("sticky") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "pause" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app.state::<AppState>();
                            match state.repo.settings().await {
                                Ok(s) => {
                                    let action=if s.paused&&s.pause_until_utc.is_none_or(|u|u>now()){Action::Resume}else{Action::Pause{until:None}};
                                    if let Err(e) = state.repo.mutate(&action,&uuid_key(),now(),false).await {
                                        state.status.lock().unwrap().last_error = Some(err(e));
                                    }
                                    let _ = app.emit("settings-changed", ());
                                }
                                Err(e) => state.status.lock().unwrap().last_error = Some(err(e)),
                            }
                        });
                    }
                    id => {
                        let tab = if id == "settings" {
                            "settings"
                        } else if id == "add" {
                            "add"
                        } else {
                            "today"
                        };
                        let _ = open_dashboard(app.clone(), tab.into());
                    }
                })
                .build(app)?;
            if let Some(tab) = launch_tab(&std::env::args().collect::<Vec<_>>()) {
                let _ = open_dashboard(handle.clone(), tab);
            }
            if std::env::args().any(|arg|arg=="--test-notification") {
                let test_app=handle.clone();tauri::async_runtime::spawn(async move {
                    let before=notifications::history_count(&test_app.config().identifier).ok();
                    let result=test_notification(test_app.clone(),test_app.state::<AppState>()).await;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let after=notifications::history_count(&test_app.config().identifier).ok();
                    if let Ok(dir)=test_app.path().app_data_dir() {let _=std::fs::write(dir.join("notification-test.json"),serde_json::json!({"tested_at_utc":now(),"result":result,"history_before":before,"history_after":after}).to_string());}
                });
            }
            start_engine(handle);
            if test_profile {
                if let Some(seconds)=std::env::args().find_map(|arg|arg.strip_prefix("--test-exit-after=").and_then(|n|n.parse::<u64>().ok())).filter(|n|(1..=120).contains(n)) {
                    let app=app.handle().clone();tauri::async_runtime::spawn(async move{tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;app.exit(0);});
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_tasks,
            task_origin,
            set_step_estimate,
            create_task,
            update_task,
            snooze_task,
            unsnooze_task,
            archive_task,
            restore_task,
            get_settings,
            save_settings,
            engine_status,
            test_notification,
            chat,
            confirm_batch,undo_batch,list_goals,save_goal,goal_steps,dismiss_step,add_goal_step,replan,ai_key,ai_test,codex_status,codex_login,save_draft,load_draft,
            confirm_chat,
            chat_history,
            clear_chat,
            export_backup,
            save_backup_file,
            save_sqlite_backup,
            preview_import,
            import_backup,
            open_dashboard,
            hide_window,
            collapse_note,
            reset_position,
            initial_tab
        ])
        .run(context);
    if let Err(error) = result {
        if let Some(root)=diagnostic_root {
            let _=std::fs::create_dir_all(&root);
            let _=std::fs::write(root.join("startup-error.txt"),error.to_string());
            return;
        }
        notifications::startup_error(&error.to_string());
    }
}

#[cfg(test)]
mod desktop_tests {
    #[test]
    fn startup_path_with_spaces_is_quoted() {
        assert_eq!(
            super::startup_command(std::path::Path::new(
                r"C:\Users\HP\AppData\Local\Task Note AI\task-note-ai.exe"
            )),
            r#""C:\Users\HP\AppData\Local\Task Note AI\task-note-ai.exe""#
        );
    }
}
