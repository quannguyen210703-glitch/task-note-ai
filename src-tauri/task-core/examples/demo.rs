use chrono::{Duration, Timelike};
use task_note_core::{Repository, Settings, TaskInput};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::memory().await?;
    let now = chrono::Utc::now().timestamp();
    let local = Settings::default().local_datetime(now);
    let base = local
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap();
    for (title, hour) in [
        ("[DEMO] Kiểm tra báo cáo", 10),
        ("[DEMO] Họp triển khai", 14),
        ("[DEMO] Chốt công việc", 16),
    ] {
        let input = TaskInput::new(title, Some((base + Duration::hours(hour)).timestamp()));
        repo.create(&input, now).await?;
    }
    std::fs::create_dir_all("artifacts")?;
    std::fs::write(
        "artifacts/demo.json",
        serde_json::to_string_pretty(&repo.export(now).await?)?,
    )?;
    println!("Đã xuất artifacts/demo.json; chưa nhập vào DB người dùng.");
    Ok(())
}
