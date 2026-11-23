use tauri::AppHandle;

#[tauri::command]
pub fn enter_mini_player(
    _app: AppHandle,
    _width: Option<f64>,
    _height: Option<f64>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn resize_mini_player(_app: AppHandle, _width: f64, _height: f64) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn exit_mini_player(_app: AppHandle) -> Result<(), String> {
    Ok(())
}
