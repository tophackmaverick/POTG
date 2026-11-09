use crate::audio::{self, AudioInput, AudioSession};
use crate::ffmpeg::{command, tool};
use crate::settings::{QualitySettings, Settings, SettingsState};
use serde::Serialize;
use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Clone, Copy)]
enum Source {
    Ddagrab,
    Gdigrab,
    TestPattern,
}

impl Source {
    fn label(self) -> &'static str {
        match self {
            Source::Ddagrab => "screen (DXGI)",
            Source::Gdigrab => "screen (GDI)",
            Source::TestPattern => "test pattern (real capture needs Windows)",
        }
    }
}

struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    stop: Arc<AtomicBool>,
    dir: PathBuf,
    log: Arc<Mutex<VecDeque<String>>>,
    encoder: String,
    source: String,
    command_line: String,
    replay_seconds: u32,
    audio: Option<AudioSession>,
    audio_label: String,
}

pub struct Recorder {
    session: Mutex<Option<Session>>,
    last_error: Mutex<String>,
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            last_error: Mutex::new(String::new()),
        }
    }
}

#[derive(Serialize)]
pub struct RecorderStatus {
    pub recording: bool,
    pub encoder: String,
    pub source: String,
    pub audio: String,
    pub buffered_seconds: u32,
    pub last_error: String,
    pub command: String,
    pub log: Vec<String>,
}

fn ffmpeg_bin() -> String {
    tool("ffmpeg", "POTG_FFMPEG")
}

fn set_error(recorder: &Recorder, message: String) {
    *recorder.last_error.lock().unwrap_or_else(|e| e.into_inner()) = message;
}

fn get_error(recorder: &Recorder) -> String {
    recorder.last_error.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn has_filter(name: &str) -> bool {
    command(&ffmpeg_bin())
        .args(["-hide_banner", "-filters"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(name))
        .unwrap_or(false)
}

fn encoder_works(encoder: &str) -> bool {
    let out = command(&ffmpeg_bin())
        .args([
            "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i",
            "color=c=black:s=256x256:r=30:d=0.4", "-vf", "format=yuv420p", "-c:v", encoder,
            "-f", "null", "-",
        ])
        .output();
    matches!(out, Ok(o) if o.status.success())
}

fn pick_encoder() -> Result<String, String> {
    let candidates: &[&str] = if cfg!(target_os = "windows") {
        &["h264_nvenc", "h264_amf", "h264_qsv", "h264_mf", "libx264"]
    } else if cfg!(target_os = "macos") {
        &["h264_videotoolbox", "libx264"]
    } else {
        &["libx264"]
    };
    candidates
        .iter()
        .find(|e| encoder_works(e))
        .map(|e| e.to_string())
        .ok_or_else(|| "FFmpeg has no working H.264 encoder on this computer".to_string())
}

fn pick_source() -> Source {
    if cfg!(target_os = "windows") {
        if has_filter("ddagrab") {
            Source::Ddagrab
        } else {
            Source::Gdigrab
        }
    } else {
        Source::TestPattern
    }
}

fn target_height(resolution: &str) -> Option<u32> {
    match resolution {
        "720p" => Some(720),
        "1080p" => Some(1080),
        "1440p" => Some(1440),
        _ => None,
    }
}

fn add(args: &mut Vec<String>, items: &[&str]) {
    args.extend(items.iter().map(|s| s.to_string()));
}

fn build_args(
    source: Source,
    encoder: &str,
    q: &QualitySettings,
    dir: &Path,
    audio_inputs: &[AudioInput],
    offset_ms: i32,
) -> Vec<String> {
    let fps = q.fps.to_string();
    let mut args: Vec<String> = Vec::new();
    add(&mut args, &["-hide_banner", "-loglevel", "warning", "-nostats"]);

    let mut filters: Vec<String> = Vec::new();
    match source {
        Source::Ddagrab => {
            add(&mut args, &["-f", "lavfi", "-i"]);
            args.push(format!("ddagrab=output_idx=0:framerate={fps}:draw_mouse=1"));
            filters.push("hwdownload".into());
            filters.push("format=bgra".into());
        }
        Source::Gdigrab => {
            add(&mut args, &["-f", "gdigrab", "-framerate", &fps, "-draw_mouse", "1", "-i", "desktop"]);
        }
        Source::TestPattern => {
            add(&mut args, &["-re", "-f", "lavfi", "-i"]);
            args.push(format!("testsrc2=size=1280x720:rate={fps}"));
        }
    }
    for input in audio_inputs {
        add(&mut args, &["-thread_queue_size", "1024", "-f", "f32le", "-ar", "48000", "-ac", "2"]);
        if offset_ms != 0 {
            args.push("-itsoffset".into());
            args.push(format!("{:.3}", offset_ms as f64 / 1000.0));
        }
        add(&mut args, &["-analyzeduration", "0", "-probesize", "32", "-i"]);
        args.push(format!("tcp://127.0.0.1:{}", input.port));
    }

    if let Some(h) = target_height(&q.resolution) {
        filters.push(format!("scale=-2:{h}"));
    }
    filters.push("format=yuv420p".into());
    args.push("-vf".into());
    args.push(filters.join(","));
    if !audio_inputs.is_empty() {
        add(&mut args, &["-map", "0:v"]);
        for index in 1..=audio_inputs.len() {
            args.push("-map".into());
            args.push(format!("{index}:a"));
        }
        add(&mut args, &["-c:a", "aac", "-b:a", "160k"]);
    }

    add(&mut args, &["-c:v", encoder]);
    match encoder {
        "libx264" => add(&mut args, &["-preset", "veryfast"]),
        "h264_videotoolbox" => add(&mut args, &["-realtime", "1"]),
        _ => {}
    }

    let rate = q.bitrate_mbps;
    args.push("-b:v".into());
    args.push(format!("{rate}M"));
    args.push("-maxrate".into());
    args.push(format!("{rate}M"));
    args.push("-bufsize".into());
    args.push(format!("{}M", rate * 2));

    args.push("-g".into());
    args.push(fps);
    add(&mut args, &["-force_key_frames", "expr:gte(t,n_forced*1)"]);
    add(
        &mut args,
        &["-f", "segment", "-segment_time", "1", "-segment_format", "mpegts", "-reset_timestamps", "1"],
    );
    args.push(dir.join("seg_%06d.ts").to_string_lossy().into_owned());
    args
}

fn segments(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("ts"))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

fn prune(dir: &Path, keep: usize) {
    let found = segments(dir);
    if found.len() > keep {
        for path in &found[..found.len() - keep] {
            let _ = fs::remove_file(path);
        }
    }
}

fn log_tail(log: &Arc<Mutex<VecDeque<String>>>) -> String {
    let lines = log.lock().unwrap_or_else(|e| e.into_inner());
    lines
        .iter()
        .rev()
        .take(3)
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn resolve_folder(app: &AppHandle, settings: &Settings) -> Result<PathBuf, String> {
    let folder = if settings.clips_folder.trim().is_empty() {
        app.path().video_dir().map_err(|e| e.to_string())?.join("POTG")
    } else {
        PathBuf::from(&settings.clips_folder)
    };
    fs::create_dir_all(&folder).map_err(|e| format!("cannot create the clips folder: {e}"))?;
    Ok(folder)
}

pub fn start(recorder: &Recorder, settings: &Settings) -> Result<(), String> {
    let mut guard = recorder.session.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() {
        return Ok(());
    }
    set_error(recorder, String::new());

    let encoder = pick_encoder().map_err(|e| {
        set_error(recorder, e.clone());
        e
    })?;
    let source = pick_source();
    let dir = std::env::temp_dir().join("potg-buffer");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create the buffer folder: {e}"))?;

    let log: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    let wants_system = settings.capture_system_audio;
    let wants_mic = settings.capture_mic;
    let audio_session = if wants_system || wants_mic {
        match audio::start(wants_system, wants_mic, &log) {
            Ok(session) => Some(session),
            Err(e) => {
                set_error(recorder, e.clone());
                return Err(e);
            }
        }
    } else {
        None
    };
    let audio_label = match (wants_system, wants_mic) {
        (true, true) => "system audio and microphone",
        (true, false) => "system audio",
        (false, true) => "microphone",
        (false, false) => "none",
    }
    .to_string();

    let no_inputs: Vec<AudioInput> = Vec::new();
    let inputs: &[AudioInput] = audio_session
        .as_ref()
        .map(|a| a.inputs.as_slice())
        .unwrap_or(&no_inputs);
    let args = build_args(
        source,
        &encoder,
        &settings.quality,
        &dir,
        inputs,
        settings.audio_offset_ms,
    );
    let command_line = format!("{} {}", ffmpeg_bin(), args.join(" "));

    let spawned = command(&ffmpeg_bin())
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => {
            if let Some(a) = audio_session {
                a.stop();
            }
            let message = format!("could not start FFmpeg: {e}. Is it installed and on PATH?");
            set_error(recorder, message.clone());
            return Err(message);
        }
    };

    if let Some(stderr) = child.stderr.take() {
        let log = log.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let mut lines = log.lock().unwrap_or_else(|e| e.into_inner());
                lines.push_back(line);
                while lines.len() > 40 {
                    lines.pop_front();
                }
            }
        });
    }

    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        let dir = dir.clone();
        let keep = settings.quality.replay_seconds as usize + 5;
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                prune(&dir, keep);
                std::thread::sleep(Duration::from_secs(1));
            }
        });
    }

    std::thread::sleep(Duration::from_millis(1500));
    if let Ok(Some(exit)) = child.try_wait() {
        stop.store(true, Ordering::Relaxed);
        if let Some(a) = audio_session {
            a.stop();
        }
        let message = format!("FFmpeg stopped right away ({exit}). {}", log_tail(&log));
        set_error(recorder, message.clone());
        let _ = fs::remove_dir_all(&dir);
        return Err(message);
    }

    let stdin = child.stdin.take();
    *guard = Some(Session {
        child,
        stdin,
        stop,
        dir,
        log,
        encoder,
        source: source.label().to_string(),
        command_line,
        replay_seconds: settings.quality.replay_seconds,
        audio: audio_session,
        audio_label,
    });
    Ok(())
}

pub fn stop(recorder: &Recorder) {
    let taken = recorder.session.lock().unwrap_or_else(|e| e.into_inner()).take();
    let Some(mut session) = taken else { return };
    session.stop.store(true, Ordering::Relaxed);
    if let Some(mut stdin) = session.stdin.take() {
        let _ = stdin.write_all(b"q");
        let _ = stdin.flush();
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match session.child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = session.child.kill();
                let _ = session.child.wait();
                break;
            }
        }
    }
    if let Some(audio) = session.audio.take() {
        audio.stop();
    }
    let _ = fs::remove_dir_all(&session.dir);
}

fn unique_output(folder: &Path) -> PathBuf {
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let mut candidate = folder.join(format!("POTG_{stamp}.mp4"));
    let mut n = 2;
    while candidate.exists() {
        candidate = folder.join(format!("POTG_{stamp}_{n}.mp4"));
        n += 1;
    }
    candidate
}

pub fn save(app: &AppHandle, recorder: &Recorder, settings: &Settings) -> Result<PathBuf, String> {
    let (dir, replay, has_audio) = {
        let guard = recorder.session.lock().unwrap_or_else(|e| e.into_inner());
        let session = guard
            .as_ref()
            .ok_or_else(|| "POTG is not recording. Press Start recording first.".to_string())?;
        (session.dir.clone(), session.replay_seconds, session.audio.is_some())
    };

    let found = segments(&dir);
    if found.len() < 2 {
        return Err("Not enough footage yet. Try again in a moment.".into());
    }
    let take = (replay as usize + 1).min(found.len());
    let mut chosen: Vec<PathBuf> = found[found.len() - take..].to_vec();
    if chosen.len() > 1 {
        let tiny = chosen
            .last()
            .and_then(|p| fs::metadata(p).ok())
            .map(|m| m.len() < 8192)
            .unwrap_or(true);
        if tiny {
            chosen.pop();
        }
    }

    let folder = resolve_folder(app, settings)?;
    let output = unique_output(&folder);

    let list_path = dir.join("concat.txt");
    let list: String = chosen
        .iter()
        .map(|p| format!("file '{}'\n", p.to_string_lossy().replace('\\', "/").replace('\'', "'\\''")))
        .collect();
    fs::write(&list_path, list).map_err(|e| format!("cannot prepare the clip: {e}"))?;

    let mut save_cmd = command(&ffmpeg_bin());
    save_cmd
        .args(["-y", "-v", "error", "-f", "concat", "-safe", "0", "-i"])
        .arg(&list_path)
        .args(["-map", "0", "-c", "copy"]);
    if has_audio {
        save_cmd.args(["-bsf:a", "aac_adtstoasc"]);
    }
    let result = save_cmd
        .args(["-movflags", "+faststart"])
        .arg(&output)
        .output()
        .map_err(|e| format!("could not start FFmpeg: {e}"))?;
    let _ = fs::remove_file(&list_path);
    if !result.status.success() {
        return Err(format!(
            "saving the clip failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    Ok(output)
}

pub fn save_from_hotkey(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let recorder = app.state::<Recorder>();
        let settings = app.state::<SettingsState>().get();
        match save(&app, recorder.inner(), &settings) {
            Ok(path) => {
                let _ = app.emit("clip-saved", path.to_string_lossy().to_string());
            }
            Err(e) => {
                let _ = app.emit("clip-failed", e);
            }
        }
    });
}

pub fn status(recorder: &Recorder) -> RecorderStatus {
    let mut guard = recorder.session.lock().unwrap_or_else(|e| e.into_inner());
    let mut died: Option<String> = None;
    if let Some(session) = guard.as_mut() {
        if let Ok(Some(exit)) = session.child.try_wait() {
            session.stop.store(true, Ordering::Relaxed);
            if let Some(audio) = session.audio.take() {
                audio.stop();
            }
            let _ = fs::remove_dir_all(&session.dir);
            died = Some(format!("FFmpeg stopped unexpectedly ({exit}). {}", log_tail(&session.log)));
        }
    }
    if let Some(message) = died {
        *guard = None;
        set_error(recorder, message);
    }

    match guard.as_ref() {
        Some(session) => RecorderStatus {
            recording: true,
            encoder: session.encoder.clone(),
            source: session.source.clone(),
            audio: session.audio_label.clone(),
            buffered_seconds: segments(&session.dir).len() as u32,
            last_error: get_error(recorder),
            command: session.command_line.clone(),
            log: session
                .log
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .cloned()
                .collect(),
        },
        None => RecorderStatus {
            recording: false,
            encoder: String::new(),
            source: String::new(),
            audio: String::new(),
            buffered_seconds: 0,
            last_error: get_error(recorder),
            command: String::new(),
            log: Vec::new(),
        },
    }
}

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    recorder: State<'_, Recorder>,
    settings: State<'_, SettingsState>,
) -> Result<(), String> {
    start(recorder.inner(), &settings.get())?;
    let _ = app.emit("recorder-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(app: AppHandle, recorder: State<'_, Recorder>) -> Result<(), String> {
    stop(recorder.inner());
    let _ = app.emit("recorder-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn save_clip_now(
    app: AppHandle,
    recorder: State<'_, Recorder>,
    settings: State<'_, SettingsState>,
) -> Result<String, String> {
    let path = save(&app, recorder.inner(), &settings.get())?;
    let text = path.to_string_lossy().to_string();
    let _ = app.emit("clip-saved", text.clone());
    Ok(text)
}

#[tauri::command]
pub fn recorder_status(recorder: State<'_, Recorder>) -> RecorderStatus {
    status(recorder.inner())
}

#[tauri::command]
pub fn default_clips_folder(app: AppHandle) -> Result<String, String> {
    let folder = resolve_folder(&app, &Settings::default())?;
    Ok(folder.to_string_lossy().into_owned())
}
