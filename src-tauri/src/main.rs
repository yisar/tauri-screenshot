use tauri::{Manager, Emitter, LogicalSize};
use std::thread;
use std::time::Duration;
use regex::{Captures, Regex};
use std::fs;
use std::path::{Path, PathBuf};
use base64::{Engine as _, engine::general_purpose};


fn inline_img(html: &str) -> String {
    // 匹配 <img ... src="...">
    let re = Regex::new(r#"(<img\b[^>]*?\bsrc\s*=\s*["'])([^"']+)(["'][^>]*>)"#).unwrap();

    re.replace_all(html, |caps: &Captures| {
        let prefix = &caps[1];
        let src = &caps[2];
        let suffix = &caps[3];

        // 已经是 base64 或网络图片
        if src.starts_with("data:")
            || src.starts_with("http://")
            || src.starts_with("https://")
        {
            return caps[0].to_string();
        }

        let path = PathBuf::from(src);

        match fs::read(&path) {
            Ok(bytes) => {
                let mime = mime_guess::from_path(&path)
                    .first_or_octet_stream();

                let encoded = general_purpose::STANDARD.encode(bytes);

                format!(
                    "{}data:{};base64,{}{}",
                    prefix,
                    mime,
                    encoded,
                    suffix
                )
            }
            Err(_) => {
                // 文件不存在则保持原样
                caps[0].to_string()
            }
        }
    })
    .into_owned()
}

#[tauri::command]
fn update_preview(app: tauri::AppHandle, mut payload: serde_json::Value) {
    let preview = app.get_webview_window("preview").unwrap();
    
    if let Some(html) = payload.get("html").and_then(|v| v.as_str()) {
        let new_html = inline_img(html);
        payload["html"] = serde_json::Value::String(new_html);
    }

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
}


// fn remove_window_corners(window: &tauri::WebviewWindow) {
//     // 获取窗口句柄
//     if let Ok(hwnd) = window.hwnd() {
//         let hwnd = HWND(hwnd.0);
//         // 设置圆角偏好为“不圆角”（值为 2）
//         let preference = DWMWCP_DONOTROUND;
//         unsafe {
//             let _ = DwmSetWindowAttribute(
//                 hwnd,
//                 DWMWA_WINDOW_CORNER_PREFERENCE,
//                 std::mem::transmute(&preference),
//                 std::mem::size_of::<DWMWCP_DONOTROUND>() as u32,
//             );
//         }
//     }
// }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![update_preview])
        .setup(|app| {
            // 1. 处理主窗口
            let main_window = app.get_webview_window("main").unwrap();

            // 2. 创建并处理预览窗口
            let preview_window = tauri::WebviewWindowBuilder::new(
                app,
                "preview",
                tauri::WebviewUrl::App("preview.html".into()),
            )
            .title("Preview")
            .decorations(false)
            .transparent(false) // 去掉边框
            .shadow(false) // 去掉边框
            .build()?;


            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri app");
}