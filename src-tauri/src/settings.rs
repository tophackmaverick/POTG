use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct QualitySettings {
    pub preset: String,
    pub resolution: String,
    pub fps: u32,
    pub bitrate_mbps: u32,
    pub replay_seconds: u32,
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self {
            preset: "medium".into(),
            resolution: "1080p".into(),
            fps: 60,
            bitrate_mbps: 20,
            replay_seconds: 60,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub quality: QualitySettings,
    pub overlay_enabled: bool,
    pub clip_hotkey: String,
    pub overlay_hotkey: String,
    pub clips_folder: String,
    pub capture_system_audio: bool,
    pub capture_mic: bool,
    pub audio_offset_ms: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            quality: QualitySettings::default(),
            overlay_enabled: true,
            clip_hotkey: "Ctrl+Shift+F9".into(),
            overlay_hotkey: "Ctrl+Shift+F10".into(),
            clips_folder: String::new(),
            capture_system_audio: true,
            capture_mic: false,
            audio_offset_ms: 0,
        }
    }
}

pub struct SettingsState(pub Mutex<Settings>);

impl SettingsState {
    pub fn new(settings: Settings) -> Self {
        Self(Mutex::new(settings))
    }

    pub fn get(&self) -> Settings {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set(&self, settings: Settings) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = settings;
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create settings folder: {e}"))?;
    Ok(dir.join("settings.json"))
}

pub fn validate(s: &Settings) -> Result<(), String> {
    let q = &s.quality;
    if !["low", "medium", "high", "custom"].contains(&q.preset.as_str()) {
        return Err("unknown quality preset".into());
    }
    if !["720p", "1080p", "1440p", "native"].contains(&q.resolution.as_str()) {
        return Err("unknown resolution".into());
    }
    if q.fps != 30 && q.fps != 60 {
        return Err("frame rate must be 30 or 60".into());
    }
    if !(2..=100).contains(&q.bitrate_mbps) {
        return Err("bitrate must be between 2 and 100 Mbps".into());
    }
    if !(15..=120).contains(&q.replay_seconds) {
        return Err("replay length must be between 15 and 120 seconds".into());
    }
    for (name, key) in [("clip", &s.clip_hotkey), ("overlay", &s.overlay_hotkey)] {
        if key.split('+').count() < 2 {
            return Err(format!("the {name} hotkey needs at least one modifier key"));
        }
    }
    if !s.clips_folder.is_empty() && !std::path::Path::new(&s.clips_folder).is_absolute() {
        return Err("the clips folder must be a full path".into());
    }
    if !(-2000..=2000).contains(&s.audio_offset_ms) {
        return Err("the audio delay must be between -2000 and 2000 ms".into());
    }
    if s.clip_hotkey.eq_ignore_ascii_case(&s.overlay_hotkey) {
        return Err("the clip hotkey and the overlay hotkey must be different".into());
    }
    Ok(())
}

pub fn load(app: &AppHandle) -> Settings {
    settings_path(app)
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .filter(|s| validate(s).is_ok())
        .unwrap_or_default()
}

fn persist(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app)?;
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("cannot save settings: {e}"))
}

fn store(app: &AppHandle, state: &SettingsState, next: Settings) -> Result<Settings, String> {
    validate(&next)?;
    let previous = state.get();
    if let Err(e) = crate::hotkeys::apply(app, &next) {
        let _ = crate::hotkeys::apply(app, &previous);
        return Err(e);
    }
    if let Err(e) = persist(app, &next) {
        let _ = crate::hotkeys::apply(app, &previous);
        return Err(e);
    }
    state.set(next.clone());
    if !next.overlay_enabled {
        crate::overlay::close(app);
    }
    let _ = app.emit("settings-changed", next.clone());
    Ok(next)
}

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Settings {
    state.get()
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    settings: Settings,
) -> Result<Settings, String> {
    store(&app, state.inner(), settings)
}

#[tauri::command]
pub async fn reset_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
) -> Result<Settings, String> {
    store(&app, state.inner(), Settings::default())
}
