use crate::ffmpeg::{command, duration_of, tool};

const SCORE_HZ: usize = 4;
const SAMPLE_RATE: usize = 16_000;
const FRAME_W: usize = 160;
const FRAME_H: usize = 90;

pub struct Highlight {
    pub peak: f64,
    pub start: f64,
    pub end: f64,
    pub confidence: f64,
}

fn run_ffmpeg(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = command(&tool("ffmpeg", "POTG_FFMPEG"))
        .args(args)
        .output()
        .map_err(|e| format!("could not start ffmpeg: {e}. Is it installed and on PATH?"))?;
    if !out.status.success() {
        return Err(format!(
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out.stdout)
}

fn audio_energy(input: &str) -> Vec<f32> {
    let args = [
        "-v", "error", "-i", input, "-vn", "-ac", "1", "-ar", "16000", "-f", "s16le", "-",
    ];
    let Ok(bytes) = run_ffmpeg(&args) else {
        return Vec::new();
    };
    let hop = SAMPLE_RATE / SCORE_HZ;
    let samples: Vec<f32> = bytes
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
        .collect();
    samples
        .chunks_exact(hop)
        .map(|window| (window.iter().map(|s| s * s).sum::<f32>() / hop as f32).sqrt())
        .collect()
}

fn motion_energy(input: &str) -> Result<Vec<f32>, String> {
    let filter = format!("fps={SCORE_HZ},scale={FRAME_W}:{FRAME_H},format=gray");
    let args = [
        "-v", "error", "-i", input, "-an", "-vf", &filter, "-f", "rawvideo", "-",
    ];
    let bytes = run_ffmpeg(&args)?;
    let frame_len = FRAME_W * FRAME_H;
    let mut energy = Vec::new();
    let mut previous: Option<&[u8]> = None;
    for current in bytes.chunks_exact(frame_len) {
        let value = match previous {
            None => 0.0,
            Some(prev) => {
                let total: u32 = current
                    .iter()
                    .zip(prev.iter())
                    .map(|(a, b)| a.abs_diff(*b) as u32)
                    .sum();
                total as f32 / frame_len as f32
            }
        };
        energy.push(value);
        previous = Some(current);
    }
    Ok(energy)
}

fn sorted(values: &[f32]) -> Vec<f32> {
    let mut copy = values.to_vec();
    copy.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    copy
}

fn percentile(values: &[f32], q: f32) -> f32 {
    let ordered = sorted(values);
    let pos = q * (ordered.len() - 1) as f32;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    ordered[lo] + (ordered[hi] - ordered[lo]) * (pos - lo as f32)
}

fn normalize(values: &[f32]) -> Vec<f32> {
    if values.is_empty() {
        return Vec::new();
    }
    let high = percentile(values, 0.98);
    if high <= 0.0 {
        return vec![0.0; values.len()];
    }
    values.iter().map(|v| (v / high).clamp(0.0, 1.0)).collect()
}

fn smooth(values: &[f32], seconds: f32) -> Vec<f32> {
    let width = ((seconds * SCORE_HZ as f32) as usize).max(1);
    let offset = ((width - 1) / 2) as isize;
    (0..values.len())
        .map(|i| {
            let mut sum = 0.0;
            for k in 0..width {
                let idx = i as isize + offset - k as isize;
                if idx >= 0 && (idx as usize) < values.len() {
                    sum += values[idx as usize];
                }
            }
            sum / width as f32
        })
        .collect()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn analyze(
    input: &str,
    pre: f64,
    post: f64,
    min_len: f64,
    max_len: f64,
) -> Result<Highlight, String> {
    let motion = motion_energy(input)?;
    let n = motion.len();
    if n < SCORE_HZ * 2 {
        return Err("clip is too short to analyze".into());
    }
    let hz = SCORE_HZ as f64;
    let duration = duration_of(input).unwrap_or(n as f64 / hz);

    let mut audio = audio_energy(input);
    let has_audio = audio.iter().any(|v| *v > 0.0);
    audio.resize(n, 0.0);

    let audio_norm = normalize(&audio);
    let motion_norm = normalize(&motion);
    let (audio_weight, motion_weight) = if has_audio { (0.6, 0.4) } else { (0.0, 1.0) };
    let combined: Vec<f32> = audio_norm
        .iter()
        .zip(motion_norm.iter())
        .map(|(a, m)| audio_weight * a + motion_weight * m)
        .collect();
    let score = smooth(&combined, 1.0);

    let mut peak_i = 0;
    let mut peak_val = f32::MIN;
    for (i, value) in score.iter().enumerate() {
        if *value > peak_val {
            peak_val = *value;
            peak_i = i;
        }
    }
    let peak_t = peak_i as f64 / hz;
    let threshold = 0.5 * peak_val;
    let half = max_len / 2.0;

    let mut lo = peak_i;
    while lo > 0 && score[lo - 1] >= threshold && ((peak_i - lo) as f64) / hz < half {
        lo -= 1;
    }
    let mut hi = peak_i;
    while hi + 1 < n && score[hi + 1] >= threshold && ((hi - peak_i) as f64) / hz < half {
        hi += 1;
    }

    let mut start = (lo as f64 / hz - pre).max(0.0);
    let mut end = (hi as f64 / hz + post).min(duration);
    if end - start < min_len {
        let pad = (min_len - (end - start)) / 2.0;
        start = (start - pad).max(0.0);
        end = (end + pad).min(duration);
    }
    if end - start > max_len {
        start = (peak_t - max_len / 2.0).max(0.0);
        end = (start + max_len).min(duration);
    }

    let ordered = sorted(&score);
    let mid = ordered.len() / 2;
    let median = if ordered.len() % 2 == 1 {
        ordered[mid]
    } else {
        (ordered[mid - 1] + ordered[mid]) / 2.0
    };
    let confidence = if peak_val > 0.0 {
        ((peak_val - median) / peak_val).clamp(0.0, 1.0) as f64
    } else {
        0.0
    };

    Ok(Highlight {
        peak: round2(peak_t),
        start: round2(start),
        end: round2(end),
        confidence: round2(confidence),
    })
}
