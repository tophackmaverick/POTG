use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::UNIX_EPOCH;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    cmd
}

fn run(mut cmd: Command, what: &str) -> Result<Output, String> {
    let out = cmd
        .output()
        .map_err(|e| format!("could not start {what}: {e}. Is it installed and on PATH?"))?;
    if !out.status.success() {
        return Err(format!(
            "{what} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out)
}

async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("background task failed: {e}"))?
}

fn unique_path(input: &Path, suffix: &str) -> PathBuf {
    let dir = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("clip");
    let mut candidate = dir.join(format!("{stem}_{suffix}.mp4"));
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{stem}_{suffix}_{n}.mp4"));
        n += 1;
    }
    candidate
}

fn check_input(input: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(input);
    if !path.is_file() {
        return Err(format!("file not found: {input}"));
    }
    Ok(path)
}

#[derive(Serialize)]
pub struct ClipInfo {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub modified_unix: u64,
}

#[tauri::command]
pub async fn list_clips(folder: String) -> Result<Vec<ClipInfo>, String> {
    blocking(move || {
        let entries =
            std::fs::read_dir(&folder).map_err(|e| format!("cannot read folder: {e}"))?;
        let mut clips = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());
            if !matches!(ext.as_deref(), Some("mp4" | "mkv" | "mov" | "webm")) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let modified_unix = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            clips.push(ClipInfo {
                path: path.to_string_lossy().into_owned(),
                name: entry.file_name().to_string_lossy().into_owned(),
                size_bytes: meta.len(),
                modified_unix,
            });
        }
        clips.sort_by(|a, b| b.modified_unix.cmp(&a.modified_unix));
        Ok(clips)
    })
    .await
}

#[tauri::command]
pub async fn get_duration(input: String) -> Result<f64, String> {
    blocking(move || {
        check_input(&input)?;
        let mut cmd = command(&env_or("POTG_FFPROBE", "ffprobe"));
        cmd.args([
            "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", &input,
        ]);
        let out = run(cmd, "ffprobe")?;
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse::<f64>()
            .map_err(|_| "could not read the video duration".to_string())
    })
    .await
}

#[tauri::command]
pub async fn make_thumbnail(input: String, at_seconds: f64) -> Result<String, String> {
    blocking(move || {
        check_input(&input)?;
        let dir = std::env::temp_dir().join("potg-thumbs");
        std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create cache folder: {e}"))?;
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        at_seconds.to_bits().hash(&mut hasher);
        let out_path = dir.join(format!("{:016x}.jpg", hasher.finish()));
        if out_path.exists() {
            return Ok(out_path.to_string_lossy().into_owned());
        }
        let mut cmd = command(&env_or("POTG_FFMPEG", "ffmpeg"));
        cmd.args(["-y", "-v", "error", "-ss", &format!("{at_seconds:.3}"), "-i", &input])
            .args(["-frames:v", "1", "-vf", "scale=320:-1"])
            .arg(&out_path);
        run(cmd, "ffmpeg")?;
        Ok(out_path.to_string_lossy().into_owned())
    })
    .await
}

fn trim_blocking(
    input: &str,
    start: f64,
    end: f64,
    output: Option<String>,
    precise: bool,
) -> Result<String, String> {
    let in_path = check_input(input)?;
    if start < 0.0 || end <= start {
        return Err("end time must be after start time".into());
    }
    let out_path = match output {
        Some(p) => PathBuf::from(p),
        None => unique_path(&in_path, "trim"),
    };
    if out_path == in_path {
        return Err("output file must be different from the original".into());
    }

    let mut cmd = command(&env_or("POTG_FFMPEG", "ffmpeg"));
    cmd.args(["-y", "-v", "error", "-ss", &format!("{start:.3}"), "-i", input])
        .args(["-t", &format!("{:.3}", end - start)]);
    if precise {
        cmd.args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "18", "-c:a", "aac"]);
    } else {
        cmd.args(["-c", "copy", "-avoid_negative_ts", "make_zero"]);
    }
    cmd.args(["-movflags", "+faststart"]).arg(&out_path);
    run(cmd, "ffmpeg")?;
    Ok(out_path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn trim_clip(
    input: String,
    start: f64,
    end: f64,
    output: Option<String>,
    precise: bool,
) -> Result<String, String> {
    blocking(move || trim_blocking(&input, start, end, output, precise)).await
}

#[derive(Deserialize)]
struct AnalysisJson {
    ok: bool,
    error: Option<String>,
    peak: Option<f64>,
    start: Option<f64>,
    end: Option<f64>,
    confidence: Option<f64>,
}

#[derive(Serialize)]
pub struct AutoCutResult {
    pub output: String,
    pub peak: f64,
    pub start: f64,
    pub end: f64,
    pub confidence: f64,
}

fn analysis_script() -> PathBuf {
    if let Ok(p) = std::env::var("POTG_ANALYSIS") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("analysis")
        .join("analyze.py")
}

#[tauri::command]
pub async fn auto_cut(input: String, pre: f64, post: f64) -> Result<AutoCutResult, String> {
    blocking(move || {
        check_input(&input)?;
        let default_python = if cfg!(windows) { "python" } else { "python3" };
        let mut cmd = command(&env_or("POTG_PYTHON", default_python));
        cmd.arg(analysis_script())
            .arg(&input)
            .args(["--pre", &pre.to_string(), "--post", &post.to_string()]);

        let out = cmd
            .output()
            .map_err(|e| format!("could not start Python: {e}. Is it installed?"))?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let parsed: AnalysisJson = serde_json::from_str(stdout.trim()).map_err(|_| {
            format!(
                "analysis gave unreadable output: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )
        })?;
        if !parsed.ok {
            return Err(parsed.error.unwrap_or_else(|| "analysis failed".into()));
        }
        let (Some(peak), Some(start), Some(end), Some(confidence)) =
            (parsed.peak, parsed.start, parsed.end, parsed.confidence)
        else {
            return Err("analysis result was incomplete".into());
        };

        let in_path = PathBuf::from(&input);
        let out_path = unique_path(&in_path, "autocut");
        let output = trim_blocking(
            &input,
            start,
            end,
            Some(out_path.to_string_lossy().into_owned()),
            true,
        )?;
        Ok(AutoCutResult { output, peak, start, end, confidence })
    })
    .await
}
