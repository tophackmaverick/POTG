mod capture;
mod ffmpeg;
mod hotkeys;
mod overlay;
mod settings;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let loaded = settings::load(app.handle());
            app.manage(settings::SettingsState::new(loaded.clone()));
            app.manage(capture::Recorder::new());
            if let Err(e) = hotkeys::apply(app.handle(), &loaded) {
                eprintln!("hotkeys: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ffmpeg::list_clips,
            ffmpeg::get_duration,
            ffmpeg::make_thumbnail,
            ffmpeg::trim_clip,
            ffmpeg::auto_cut,
            settings::get_settings,
            settings::save_settings,
            settings::reset_settings,
            overlay::close_overlay,
            capture::start_recording,
            capture::stop_recording,
            capture::save_clip_now,
            capture::recorder_status,
            capture::default_clips_folder,
        ])
        .build(tauri::generate_context!())
        .expect("error while building POTG");

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            capture::stop(handle.state::<capture::Recorder>().inner());
        }
    });
}
