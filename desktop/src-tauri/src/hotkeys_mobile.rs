use tauri::{AppHandle, Runtime};

#[tauri::command]
pub fn set_global_hotkeys<R: Runtime>(
    _app: AppHandle<R>,
    _payload: Vec<(String, String)>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn clear_global_hotkeys<R: Runtime>(_app: AppHandle<R>) -> Result<(), String> {
    Ok(())
}
