#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod library;

use tauri::{menu::Menu, Manager};

fn set_window_top(window: &tauri::WebviewWindow, enabled: bool) -> Result<bool, String> {
    window
        .set_always_on_top(enabled)
        .map_err(|e| e.to_string())?;
    // macOS applies the window level asynchronously. An immediate getter can
    // still report the previous level; acknowledge the accepted request instead.
    Ok(enabled)
}

#[tauri::command]
fn window_top_get(window: tauri::WebviewWindow) -> Result<bool, String> {
    library::allowed(&window)?;
    window.is_always_on_top().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_top_set(window: tauri::WebviewWindow, enabled: bool) -> Result<bool, String> {
    library::allowed(&window)?;
    set_window_top(&window, enabled)
}

#[tauri::command]
fn window_reload(window: tauri::WebviewWindow) -> Result<(), String> {
    library::allowed(&window)?;
    window.reload().map_err(|e| e.to_string())
}

fn main() {
    let context = tauri::generate_context!();
    let dev_origin = if cfg!(debug_assertions) {
        context
            .config()
            .build
            .dev_url
            .as_ref()
            .map(|url| url.origin().ascii_serialization())
    } else {
        None
    };
    let desktop_script = include_str!("../../desktop/desktop-interface.js").replace(
        "__DESKTOP_DEV_ORIGIN__",
        &serde_json::to_string(&dev_origin).expect("serialize development origin"),
    );
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri::plugin::Builder::<tauri::Wry>::new("desktop-library")
                .js_init_script(desktop_script)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            window_top_get,
            window_reload,
            window_top_set,
            library::library_list,
            library::library_choose,
            library::library_open,
            library::library_pin,
            library::library_remove,
            library::library_read,
        ])
        .setup(|app| {
            let storage = app.path().app_data_dir()?.join("recent-library.json");
            app.manage(std::sync::Mutex::new(
                library::Library::load(storage).map_err(std::io::Error::other)?,
            ));
            app.set_menu(Menu::default(app.handle())?)?;
            Ok(())
        })
        .run(context)
        .expect("failed to run Tetorica VGM Analyzer");
}
