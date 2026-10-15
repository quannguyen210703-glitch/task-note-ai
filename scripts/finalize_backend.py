from pathlib import Path
p=Path('src-tauri/task-core/src/service.rs')
s=p.read_text(encoding='utf-8')
s=s.replace('"scheduler_running":gui,','"scheduler_running":gui&&scheduler_running(root),')
if 'fn scheduler_running(root:' not in s:
    s+='''
fn scheduler_running(root:&Path)->bool {
    std::fs::read_to_string(root.join("engine-status.json")).ok()
        .and_then(|v|serde_json::from_str::<Value>(&v).ok())
        .is_some_and(|v|v["running"].as_bool()==Some(true)&&v["last_check"].as_i64().is_some_and(|t|(chrono::Utc::now().timestamp()-t).abs()<65))
}
'''
p.write_text(s,encoding='utf-8')
