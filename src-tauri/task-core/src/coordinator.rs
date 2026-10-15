//! One process owns migrations/writes/scheduler. File IPC inherits the private
//! current-user data directory ACL; no TCP port or public server is opened.
use crate::{service::*, *};
use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};
pub struct Owner {
    _file: File,
}
pub fn data_root() -> Result<PathBuf> {
    std::env::var_os("APPDATA")
        .map(|p| PathBuf::from(p).join("vn.tasknoteai.desktop"))
        .ok_or_else(|| {
            Error::Invalid(
                "Không xác định được AppData của người dùng; dừng để tránh tạo DB sai profile."
                    .into(),
            )
        })
}
pub fn try_owner(root: &Path) -> Result<Option<Owner>> {
    std::fs::create_dir_all(root)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("core.lock"))?;
    match file.try_lock_exclusive() {
        Ok(()) => Ok(Some(Owner { _file: file })),
        Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.raw_os_error() == fs2::lock_contended_error().raw_os_error() =>
        {
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}
pub fn legacy_active(root: &Path) -> bool {
    let path=root.join("engine-status.json");
    if !path.exists(){return false;}
    // A stale V1 heartbeat after sleep is NOT proof that its writer stopped.
    // Fail closed on partial/old metadata until V2 GUI owns and upgrades it.
    std::fs::read_to_string(path).ok()
        .and_then(|v|serde_json::from_str::<serde_json::Value>(&v).ok())
        .is_none_or(|v|v["version"].as_str()!=Some(env!("CARGO_PKG_VERSION")))
}
pub async fn send(root: &Path, req: &Request) -> Result<Response> {
    let dir = root.join("cli-ipc");
    std::fs::create_dir_all(&dir)?;
    let token = uuid::Uuid::new_v4().to_string();
    let pending = dir.join(format!("{token}.tmp"));
    let input = dir.join(format!("{token}.request"));
    let output = dir.join(format!("{token}.response"));
    std::fs::write(&pending, serde_json::to_vec(req)?)?;
    std::fs::rename(pending, &input)?;
    for _ in 0..150 {
        if let Ok(bytes) = std::fs::read(&output) {
            let response = serde_json::from_slice(&bytes)?;
            let _ = std::fs::remove_file(output);
            return Ok(response);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let _ = std::fs::remove_file(input);
    Err(std::io::Error::new(std::io::ErrorKind::TimedOut,"Core chưa trả lời trong 15 giây. Lệnh có thể đã commit; thử lại cùng --request-id, không đổi nội dung.").into())
}
pub async fn drain(repo: &Repository, root: &Path) -> Result<bool> {
    let dir = root.join("cli-ipc");
    std::fs::create_dir_all(&dir)?;
    let mut changed = false;
    for entry in std::fs::read_dir(&dir)?.take(100) {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "request") {
            continue;
        }
        if std::fs::metadata(&path)?.len() > 32000 {
            let _ = std::fs::remove_file(path);
            continue;
        }
        let request: std::result::Result<Request, _> =
            serde_json::from_slice(&std::fs::read(&path)?);
        let response = match request {
            Ok(ref req) => {
                let result = repo.execute(req, root, true).await;
                changed |= result.ok && !req.dry_run && !req.action.readonly();
                result
            }
            Err(_) => Response::failure(2, "Lệnh IPC không hợp lệ".into()),
        };
        let tmp = path.with_extension("out");
        std::fs::write(&tmp, serde_json::to_vec(&response)?)?;
        std::fs::rename(tmp, path.with_extension("response"))?;
        std::fs::remove_file(path)?;
    }
    Ok(changed)
}
