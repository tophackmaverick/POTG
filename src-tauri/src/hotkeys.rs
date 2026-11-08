use crate::settings::Settings;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

fn parse(label: &str, key: &str) -> Result<Shortcut, String> {
    key.parse::<Shortcut>()
        .map_err(|e| format!("the {label} hotkey \"{key}\" is not valid: {e}"))
}

pub fn apply(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let clip = parse("clip", &settings.clip_hotkey)?;
    let overlay = if settings.overlay_enabled {
        Some(parse("overlay", &settings.overlay_hotkey)?)
    } else {
        None
    };

    let shortcuts = app.global_shortcut();
    shortcuts.unregister_all().map_err(|e| e.to_string())?;

    shortcuts
        .on_shortcut(clip, |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let _ = app.emit("clip-requested", ());
            }
        })
        .map_err(|e| {
            format!(
                "the clip hotkey {} could not be registered, another app may be using it ({e})",
                settings.clip_hotkey
            )
        })?;

    if let Some(overlay) = overlay {
        shortcuts
            .on_shortcut(overlay, |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    crate::overlay::toggle(app);
                }
            })
            .map_err(|e| {
                format!(
                    "the overlay hotkey {} could not be registered, another app may be using it ({e})",
                    settings.overlay_hotkey
                )
            })?;
    }

    Ok(())
}
