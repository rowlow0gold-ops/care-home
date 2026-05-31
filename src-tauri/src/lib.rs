//! Tauri shell for the 케어닥 desktop station.
//!
//! Online-only: all business data lives on the server (care.minhojan-world.site)
//! and is accessed from the frontend via HTTP. The desktop process keeps just
//! one native command — `save_excel` — for the native Save-As dialog used by
//! the XLSX export/download buttons. (The old local SQLite layer was removed.)

/// Open a native Save-As dialog, write `data` to the chosen path, return the path.
/// Returns Ok(None) if the user cancels.
#[tauri::command]
async fn save_excel(
    app: tauri::AppHandle,
    filename: String,
    data: Vec<u8>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();

    app.dialog()
        .file()
        .set_file_name(&filename)
        .add_filter("Excel Workbook", &["xlsx"])
        .save_file(move |path| {
            let _ = tx.send(path);
        });

    let path = rx.await.map_err(|e| e.to_string())?;

    match path {
        Some(p) => {
            let path_str = p.to_string();
            std::fs::write(&path_str, &data).map_err(|e| e.to_string())?;
            Ok(Some(path_str))
        }
        None => Ok(None),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![save_excel])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
