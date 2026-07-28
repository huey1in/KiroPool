use tauri::{Emitter, Manager};
use tracing::{error, info, warn};

#[tauri::command]
pub fn check_admin_privileges() -> Result<bool, String> {
    crate::utils::check_admin_privileges()
}

#[tauri::command]
pub fn log_error(message: String, file: Option<String>, line: Option<u32>) {
    if let (Some(file), Some(line)) = (file, line) {
        error!(target: "frontend", "{}. Location: {} line {}", message, file, line);
    } else {
        error!(target: "frontend", "{}", message);
    }
}

#[tauri::command]
pub fn log_warn(message: String, file: Option<String>, line: Option<u32>) {
    if let (Some(file), Some(line)) = (file, line) {
        warn!(target: "frontend", "{}. Location: {} line {}", message, file, line);
    } else {
        warn!(target: "frontend", "{}", message);
    }
}

#[tauri::command]
pub fn log_info(message: String) {
    info!(target: "frontend", "{}", message);
}

#[tauri::command]
pub fn open_devtools(app_handle: tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        if window.is_devtools_open() {
            window.close_devtools();
        } else {
            window.open_devtools();
        }
    }
}

#[tauri::command]
pub async fn refresh_inbound(app_handle: tauri::AppHandle) -> Result<bool, String> {
    crate::api::inbound::init_inbound_config(&app_handle)
        .await
        .map_err(|error| format!("线路刷新失败: {error}"))?;
    if let Some(window) = app_handle.get_webview_window("main") {
        window
            .emit("inbound-refreshed", ())
            .map_err(|error| format!("发送线路刷新事件失败: {error}"))?;
    }
    Ok(true)
}
