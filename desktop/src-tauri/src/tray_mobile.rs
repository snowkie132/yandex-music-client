use tauri::{AppHandle, Runtime};

pub fn close_to_tray() -> bool {
    false
}

pub fn hide<R: Runtime>(_app: &AppHandle<R>) {}

pub fn build<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> {
    Ok(())
}

#[tauri::command]
pub fn set_close_to_tray(_enabled: bool) {}

#[tauri::command]
pub fn set_tray_tooltip(_app: AppHandle, _text: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn hide_to_tray(_app: AppHandle) {}

#[tauri::command]
pub fn show_from_tray(_app: AppHandle) {}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
