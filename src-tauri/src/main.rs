use tauri::{Manager, Emitter, LogicalSize};
use std::thread;
use std::time::Duration;


#[tauri::command]
fn update_preview(app: tauri::AppHandle, payload: serde_json::Value) {
    let preview = app.get_webview_window("preview").unwrap();

    let js = format!(
        "window.__UPDATE_PREVIEW__({})",
        serde_json::to_string(&payload).unwrap()
    );
    if let (Some(w), Some(h)) = (
        payload.get("width").and_then(|v| v.as_f64()),
        payload.get("height").and_then(|v| v.as_f64()),
    ) {
        let _ = preview.set_size(
            tauri::Size::Logical(LogicalSize {
                width: w,
                height: h,
            })
        );
    }

    preview.eval(&js).unwrap();
    // 接下来需要截图
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![update_preview])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // 创建预览窗口
            tauri::WebviewWindowBuilder::new(
                app,
                "preview",
                tauri::WebviewUrl::App("preview.html".into()),
            )
            .title("Preview")
            .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri app");
}