#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{CheckMenuItem, Menu, Submenu},
    Manager,
};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let pin = CheckMenuItem::with_id(
                app,
                "pin",
                "PIN — Always on Top",
                true,
                false,
                None::<&str>,
            )?;
            let menu = Menu::default(app.handle())?;
            menu.append(&Submenu::with_items(app, "Analyzer", true, &[&pin])?)?;
            app.set_menu(menu)?;
            app.on_menu_event(move |app, event| {
                if event.id().as_ref() != "pin" {
                    return;
                }
                if let Some(window) = app.get_webview_window("main") {
                    let result = (|| -> tauri::Result<()> {
                        let was_pinned = window.is_always_on_top()?;
                        if let Err(error) = window.set_always_on_top(!was_pinned) {
                            let _ = pin.set_checked(was_pinned);
                            return Err(error);
                        }
                        pin.set_checked(!was_pinned)
                    })();
                    if let Err(error) = result {
                        eprintln!("Cannot change PIN: {error}");
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Tetorica VGM Analyzer");
}
