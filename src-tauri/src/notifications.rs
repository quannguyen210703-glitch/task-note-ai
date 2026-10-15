// Use synchronous WinRT submission on Windows: Tauri's plugin dispatches asynchronously
// and cannot report OS rejection or apply its iOS-only `silent` flag on Windows.
#[cfg(windows)]
struct WinRtGuard(bool);
#[cfg(windows)]
impl WinRtGuard {
    fn enter() -> Result<Self, String> {
        use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
        match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
            Ok(()) => Ok(Self(true)),
            Err(e) if e.code().0 as u32 == 0x80010106 => Ok(Self(false)), // existing STA is also valid
            Err(e) => Err(e.to_string()),
        }
    }
}
#[cfg(windows)]
impl Drop for WinRtGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                windows::Win32::System::WinRT::RoUninitialize();
            }
        }
    }
}
#[cfg(windows)]
pub fn permission(identifier: &str) -> Result<String, String> {
    let _guard = WinRtGuard::enter()?;
    use windows::{
        core::HSTRING,
        UI::Notifications::{NotificationSetting, ToastNotificationManager},
    };
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(identifier))
        .map_err(|e| e.to_string())?;
    let value = match notifier.Setting() {
        Ok(value) => value,
        // Windows creates its per-app sender record on the first Show. A valid
        // installed Start-menu shortcut can therefore have no Setting yet.
        Err(error) if error.code().0 as u32 == 0x80070490 => return Ok("NotInitialized".into()),
        Err(error) => return Err(error.to_string()),
    };
    Ok(match value {
        NotificationSetting::Enabled => "Enabled",
        NotificationSetting::DisabledForApplication => "DisabledForApplication",
        NotificationSetting::DisabledForUser => "DisabledForUser",
        NotificationSetting::DisabledByGroupPolicy => "DisabledByGroupPolicy",
        NotificationSetting::DisabledByManifest => "DisabledByManifest",
        _ => "Unknown",
    }
    .into())
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
#[cfg(windows)]
pub fn fullscreen() -> bool {
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    matches!(
        unsafe { SHQueryUserNotificationState() },
        Ok(QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
    )
}
#[cfg(windows)]
pub fn startup_error(message: &str) {
    use windows::{
        core::HSTRING,
        Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK},
    };
    let body=HSTRING::from(format!("Task Note AI không khởi động được. Dữ liệu hiện có được giữ nguyên.\n\n{message}\n\nKiểm tra WebView2, quyền AppData và bản sao lưu; không xóa SQLite trước khi sao lưu."));
    unsafe {
        MessageBoxW(
            None,
            &body,
            &HSTRING::from("Task Note AI · Lỗi khởi động"),
            MB_OK | MB_ICONERROR,
        );
    }
}
#[cfg(windows)]
pub fn send(identifier: &str, title: &str, body: &str, sound: bool) -> Result<(), String> {
    let _guard = WinRtGuard::enter()?;
    use windows::{
        core::HSTRING,
        Data::Xml::Dom::XmlDocument,
        UI::Notifications::{ToastNotification, ToastNotificationManager},
    };
    let xml = XmlDocument::new().map_err(|e| e.to_string())?;
    let audio = if sound {
        ""
    } else {
        "<audio silent=\"true\"/>"
    };
    xml.LoadXml(&HSTRING::from(format!("<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual>{audio}</toast>",escape(title),escape(body)))).map_err(|e|e.to_string())?;
    let toast = ToastNotification::CreateToastNotification(&xml).map_err(|e| e.to_string())?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(identifier))
        .and_then(|n| n.Show(&toast))
        .map_err(|e| e.to_string())
}
#[cfg(windows)]
pub fn history_count(identifier: &str) -> Result<u32, String> {
    let _guard = WinRtGuard::enter()?;
    use windows::{core::HSTRING, UI::Notifications::ToastNotificationManager};
    ToastNotificationManager::History()
        .and_then(|history| history.GetHistoryWithId(&HSTRING::from(identifier)))
        .and_then(|entries| entries.Size())
        .map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    #[test]
    fn xml_escape() {
        assert_eq!(
            super::escape("<Báo cáo> & \"x\""),
            "&lt;Báo cáo&gt; &amp; &quot;x&quot;"
        );
    }
}
