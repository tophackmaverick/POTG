# POTG

**Your best moments, already edited.**

POTG is a gameplay clipping app in the spirit of Medal, with one difference: Medal saves the clip, POTG finishes it. Open a clip, press **Auto Cut**, and POTG analyzes the video and trims it around the moment that matters.

> **Status: early development.** POTG currently works on video files you already have. Built-in game recording is the next big milestone and is not available yet.

Website: POTG.gg (coming soon)

## What works today

- **Clip library.** Pick a folder and browse your clips as a grid with thumbnails, duration, size, and date, plus search.
- **Player and manual trim.** Preview a clip, set start and end points with sliders, and save a trimmed copy. The original is never overwritten.
- **Auto Cut.** POTG scores the audio loudness and on-screen motion across the clip, finds the peak, and saves a trimmed copy around it with adjustable time before and after.
- **Settings.** Capture quality presets (Low, Medium, High, Custom) with resolution, frame rate, bitrate, and replay length, plus a live memory estimate.
- **Hotkeys.** Separate, rebindable shortcuts for saving a clip and for opening the overlay, with conflict warnings.
- **Overlay.** A small always-on-top window, opened by hotkey, with the same quality settings. It can be turned off completely in Settings.

## Coming next

- Built-in recording with no OBS or other apps required: Windows Graphics Capture, hardware encoding, and a rolling replay buffer saved with the clip hotkey
- Game and microphone audio capture
- Captions and 9:16 vertical export for TikTok and Shorts
- A self-contained Windows installer with FFmpeg and the analysis engine bundled in

## How Auto Cut works

1. Audio is extracted and its loudness is measured over short time slices.
2. Frames are sampled and compared to measure how much changes on screen.
3. The two signals are combined into one highlight score over time.
4. The highest peak is found, the window grows outward while the score stays high, and pre-roll and post-roll are added.
5. FFmpeg writes the trimmed clip.

## Tech stack

| Part | Technology |
|---|---|
| App shell | Tauri 2 |
| Interface | Svelte, SvelteKit, TypeScript |
| Backend | Rust |
| Analysis | Python with NumPy and OpenCV |
| Video processing | FFmpeg |

## Getting started

### Requirements

- Node.js (LTS)
- Rust, installed from rustup.rs
- Python 3.11 or newer
- FFmpeg and FFprobe on your PATH
- Windows: Visual Studio C++ Build Tools. macOS: Xcode Command Line Tools (`xcode-select --install`)

### Run it

```
git clone https://github.com/YOUR-USERNAME/potg.git
cd potg
npm install
pip install -r analysis/requirements.txt
npm run tauri dev
```

The first run compiles the Rust code and takes a few minutes. Choose a folder that contains video clips to get started.

### Try the analysis on its own

```
python analysis/analyze.py path/to/clip.mp4
```

It prints one line of JSON with the peak time and the suggested start and end.

### Environment overrides

For development you can point POTG at specific tools with `POTG_FFMPEG`, `POTG_FFPROBE`, `POTG_PYTHON`, and `POTG_ANALYSIS`.

## Default hotkeys

| Action | Shortcut |
|---|---|
| Save a clip | Ctrl + Shift + F9 |
| Open or close the overlay | Ctrl + Shift + F10 |

Both can be changed in Settings. On a Mac, function keys may need the fn key held.

## Project structure

```
analysis/        Python analysis script and requirements
src/             Svelte interface (library, editor, settings, overlay)
src-tauri/       Rust backend (trim, Auto Cut, settings, hotkeys, overlay)
```

## Roadmap

- [x] M0 Project setup
- [x] M1 Clip library and manual trim
- [x] M2 Auto Cut v1
- [ ] M2b Settings, hotkeys, and overlay
- [ ] M3 Native capture and replay buffer
- [ ] M4 Audio and captions
- [ ] M5 Hardening and compatibility
- [ ] M6 Self-contained installer and website launch

## Known limitations

- Recording is not implemented yet.
- The overlay is a normal window, so it will not appear over games in exclusive fullscreen. Borderless windowed mode is the target.
- The first installers are unsigned, so Windows SmartScreen shows a warning. Choose More info, then Run anyway.
- Early installers do not bundle FFmpeg or Python yet, so Auto Cut and thumbnails need them installed.

## Contributing

Commit messages follow Conventional Commits, for example `feat(library): add manual trim` or `fix(capture): handle black frames`.

## License

To be decided.
