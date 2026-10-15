//! Opt-in native QA fixture for the installed app. Never runs at normal startup.
use serde::{Deserialize, Serialize};
use task_note_core::{Repository, Settings, TaskInput};
#[derive(Serialize, Deserialize)]
struct Fixture {
    id: String,
    due_at_utc: i64,
    original_settings: Settings,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "setup".into());
    let dir = std::path::PathBuf::from(std::env::var("APPDATA")?).join("vn.tasknoteai.desktop");
    let db = dir.join("task-note.sqlite");
    if !db.exists() {
        return Err("Cần cài và mở Task Note AI trước; không tạo DB demo thay thế.".into());
    }
    let repo = Repository::open(&db).await?;
    let now = chrono::Utc::now().timestamp();
    std::fs::create_dir_all("artifacts")?;
    if mode == "setup" {
        if std::path::Path::new("artifacts/native-fixture.json").exists() {
            return Err("Fixture hiện có cần cleanup trước.".into());
        }
        let original_settings = repo.settings().await?;
        let mut input = TaskInput::new("[QA] Kiểm tra nhắc khi ẩn", Some(now + 120));
        input.tag_or_project = "__qa_native".into();
        let task = repo.create(&input, now).await?;
        let fixture = Fixture {
            id: task.id.clone(),
            due_at_utc: now + 120,
            original_settings: original_settings.clone(),
        };
        std::fs::write(
            "artifacts/native-fixture.json",
            serde_json::to_string_pretty(&fixture)?,
        )?;
        let mut settings = original_settings;
        settings.show_sticky = false;
        settings.notifications = true;
        settings.paused = false;
        settings.onboarding_done = true;
        repo.save_settings(&settings).await?;
        println!("Fixture={} due_at_utc={}", task.id, now + 120);
    } else if mode == "cleanup" {
        let fixture: Fixture =
            serde_json::from_str(&std::fs::read_to_string("artifacts/native-fixture.json")?)?;
        let task = repo.get(&fixture.id).await?;
        let dispatches:Vec<(String,i64,String)>=sqlx::query_as("SELECT dispatch_key,sent_at_utc,status FROM reminder_dispatches WHERE dispatch_key LIKE ? ORDER BY sent_at_utc").bind(format!("task:{}:%",fixture.id)).fetch_all(&repo.pool).await?;
        std::fs::write(
            "artifacts/native-scheduler-evidence.json",
            serde_json::to_string_pretty(
                &serde_json::json!({"id":fixture.id,"due_at_utc":fixture.due_at_utc,"persisted_after_restart":task.id==fixture.id,"dispatches":dispatches,"checked_at_utc":now}),
            )?,
        )?;
        repo.save_settings(&fixture.original_settings).await?;
        // Only the tagged UUID made by this QA helper is removed, never other tasks.
        let count = sqlx::query("DELETE FROM tasks WHERE id=? AND tag_or_project='__qa_native'")
            .bind(&fixture.id)
            .execute(&repo.pool)
            .await?
            .rows_affected();
        println!("Restored settings; removed owned QA fixture rows={count}");
    } else {
        return Err("Chỉ hỗ trợ setup hoặc cleanup.".into());
    }
    Ok(())
}
