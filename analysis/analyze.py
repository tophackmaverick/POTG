import argparse
import json
import os
import subprocess
import sys

import cv2
import numpy as np

FFMPEG = os.environ.get("POTG_FFMPEG", "ffmpeg")
SAMPLE_RATE = 16000
SCORE_HZ = 4

def extract_audio_energy(path: str) -> np.ndarray:
    cmd = [FFMPEG, "-v", "error", "-i", path, "-vn", "-ac", "1",
           "-ar", str(SAMPLE_RATE), "-f", "s16le", "-"]
    proc = subprocess.run(cmd, capture_output=True)
    if proc.returncode != 0 or not proc.stdout:
        return np.zeros(0)
    samples = np.frombuffer(proc.stdout, dtype=np.int16).astype(np.float32) / 32768.0
    hop = SAMPLE_RATE // SCORE_HZ
    n = len(samples) // hop
    if n == 0:
        return np.zeros(0)
    frames = samples[: n * hop].reshape(n, hop)
    return np.sqrt((frames ** 2).mean(axis=1))

def extract_motion(path: str) -> tuple[np.ndarray, float]:
    cap = cv2.VideoCapture(path)
    if not cap.isOpened():
        raise RuntimeError("could not open video")
    fps = cap.get(cv2.CAP_PROP_FPS) or 30.0
    total = cap.get(cv2.CAP_PROP_FRAME_COUNT) or 0
    step = max(1, int(round(fps / SCORE_HZ)))
    motion, prev, idx = [], None, 0
    while True:
        if not cap.grab():
            break
        if idx % step == 0:
            ok, frame = cap.retrieve()
            if not ok:
                break
            small = cv2.cvtColor(cv2.resize(frame, (160, 90)), cv2.COLOR_BGR2GRAY)
            motion.append(0.0 if prev is None else float(cv2.absdiff(small, prev).mean()))
            prev = small
        idx += 1
    cap.release()
    duration = (total / fps) if total else len(motion) / SCORE_HZ
    return np.array(motion, dtype=np.float32), duration

def normalize(x: np.ndarray) -> np.ndarray:
    if len(x) == 0:
        return x
    hi = np.percentile(x, 98)
    return np.clip(x / hi, 0, 1) if hi > 0 else np.zeros_like(x)

def smooth(x: np.ndarray, seconds: float) -> np.ndarray:
    w = max(1, int(seconds * SCORE_HZ))
    return np.convolve(x, np.ones(w) / w, mode="same")

def analyze(path: str, pre: float, post: float, min_len: float, max_len: float) -> dict:
    audio = extract_audio_energy(path)
    motion, duration = extract_motion(path)
    n = len(motion)
    if n < SCORE_HZ * 2:
        raise RuntimeError("clip is too short to analyze")

    audio = np.resize(audio, n) if len(audio) else np.zeros(n)
    a, m = normalize(audio), normalize(motion)
    weights = (0.6, 0.4) if len(audio) and audio.any() else (0.0, 1.0)
    score = smooth(weights[0] * a + weights[1] * m, 1.0)

    peak_i = int(np.argmax(score))
    peak_val = float(score[peak_i])
    peak_t = peak_i / SCORE_HZ

    threshold = 0.5 * peak_val
    lo = peak_i
    while lo > 0 and score[lo - 1] >= threshold and (peak_i - lo) / SCORE_HZ < max_len / 2:
        lo -= 1
    hi = peak_i
    while hi < n - 1 and score[hi + 1] >= threshold and (hi - peak_i) / SCORE_HZ < max_len / 2:
        hi += 1

    start = max(0.0, lo / SCORE_HZ - pre)
    end = min(duration, hi / SCORE_HZ + post)

    if end - start < min_len:
        pad = (min_len - (end - start)) / 2
        start, end = max(0.0, start - pad), min(duration, end + pad)
    if end - start > max_len:
        start = max(0.0, peak_t - max_len / 2)
        end = min(duration, start + max_len)

    median = float(np.median(score))
    confidence = float(np.clip((peak_val - median) / peak_val, 0, 1)) if peak_val > 0 else 0.0
    return {"ok": True, "peak": round(peak_t, 2), "start": round(start, 2),
            "end": round(end, 2), "duration": round(duration, 2),
            "confidence": round(confidence, 2)}

def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("video")
    p.add_argument("--pre", type=float, default=6.0, help="seconds kept before the action")
    p.add_argument("--post", type=float, default=4.0, help="seconds kept after the action")
    p.add_argument("--min-len", type=float, default=5.0)
    p.add_argument("--max-len", type=float, default=30.0)
    args = p.parse_args()
    try:
        result = analyze(args.video, args.pre, args.post, args.min_len, args.max_len)
        print(json.dumps(result))
        return 0
    except Exception as e:
        print(json.dumps({"ok": False, "error": str(e)}))
        return 1

if __name__ == "__main__":
    sys.exit(main())
