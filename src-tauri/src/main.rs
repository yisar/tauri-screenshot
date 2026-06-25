#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{LogicalSize, Manager};

use regex::{Captures, Regex};
use std::{
    fs,
    path::PathBuf,
    thread,
    time::Duration,
};

use base64::{engine::general_purpose, Engine as _};

use xcap::Window;

fn inline_img(html: &str) -> String {
    let re = Regex::new(r#"(<img\b[^>]*?\bsrc\s*=\s*["'])([^"']+)(["'][^>]*>)"#).unwrap();

    re.replace_all(html, |caps: &Captures| {
        let prefix = &caps[1];
        let src = &caps[2];
        let suffix = &caps[3];

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
            Err(_) => caps[0].to_string(),
        }
    })
    .into_owned()
}

/// 找到 Preview 窗口（xcap 0.9.6 safe helper）
fn find_preview_window() -> Result<xcap::Window, String> {
    let windows = Window::all().map_err(|e| e.to_string())?;
    println!("Preview window found: {:?}", "111");

    windows
        .into_iter()
        .find(|w| {
            w.title()
                .as_deref()
                .map(|t| t == "Preview")
                .unwrap_or(false)
        })
        .ok_or_else(|| "Preview window not found".to_string())
}

#[tauri::command]
fn capture_preview() -> Result<String, String> {
    let win = find_preview_window()?;


    let image = win.capture_image().map_err(|e| e.to_string())?;

    let path = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("preview.png");

    image.save(&path).map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn update_preview(
    app: tauri::AppHandle,
    mut payload: serde_json::Value,
) -> Result<(), String> {

    let preview = app
        .get_webview_window("preview")
        .ok_or_else(|| "preview window not found".to_string())?;

    // HTML inline image
    if let Some(html) = payload.get("html").and_then(|v| v.as_str()) {
        let new_html = inline_img(html);
        payload["html"] = serde_json::Value::String(new_html);
    }

    // send JS
    let js = format!(
        "window.__UPDATE_PREVIEW__({})",
        serde_json::to_string(&payload).unwrap()
    );

    // resize window if needed
    if let (Some(w), Some(h)) = (
        payload.get("width").and_then(|v| v.as_f64()),
        payload.get("height").and_then(|v| v.as_f64()),
    ) {
        preview
            .set_size(tauri::Size::Logical(LogicalSize {
                width: w,
                height: h,
            }))
            .map_err(|e| e.to_string())?;
    }

    preview.eval(&js).map_err(|e| e.to_string())?;

    // ⚠️ 这里仍然是 blocking（但保持你原逻辑）
    thread::sleep(Duration::from_millis(300));

    // screenshot via xcap 0.9.6
    let win = find_preview_window()?;

    println!("Preview window found: {:?}", win.title());
    println!("=============================");

    let image = win.capture_image().map_err(|e| e.to_string())?;

    let path = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("preview.png");

    image.save(&path).map_err(|e| e.to_string())?;

    println!("Saved screenshot: {}", path.display());

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            update_preview,
            capture_preview
        ])
        .setup(|app| {
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
        .expect("error while running tauri application");
}