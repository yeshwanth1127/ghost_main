use active_win_pos_rs::get_active_window;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ActiveWindowInfo {
    pub title: String,
    pub app_name: String,
}

#[tauri::command]
pub fn get_active_window_info() -> Option<ActiveWindowInfo> {
    get_active_window().ok().map(|w| ActiveWindowInfo {
        title: w.title,
        app_name: w.app_name,
    })
}
