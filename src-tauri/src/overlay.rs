use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn toggle(app: &AppHandle) {
    if app.get_webview_window("overlay").is_some() {
        close(app);
    } else {
        open(app);
    }
}

pub fn open(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let built = WebviewWindowBuilder::new(&handle, "overlay", WebviewUrl::App("overlay".into()))
            .title("POTG Overlay")
            .inner_size(440.0, 620.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .center()
            .focused(true)
            .build();
        if let Err(e) = built {
            eprintln!("could not open the overlay: {e}");
        }
    });
}

pub fn close(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.close();
    }
}

#[tauri::command]
pub fn close_overlay(app: AppHandle) {
    close(&app);
}
