# POTG

**Your best moments, already edited.**

POTG is a gameplay clipping app that doesn't stop at saving the clip: it finishes it. Open a clip, press **Auto Cut**, and POTG analyzes the video and trims it around the moment that matters.

> **Status: early development.** Built-in recording, with game sound and microphone, is new and still being tested on Windows. Clip editing and Auto Cut work on any video file.

Website: POTG.gg (coming soon)

## What works today

- **Recording (in testing).** Start recording, and POTG keeps a rolling replay buffer of your screen with sound. Press the clip hotkey to save the last seconds as an MP4. It picks a hardware encoder automatically.
- **Audio (in testing).** Game and system sound, and optionally your microphone as a separate track, with an adjustable audio delay for sync.
- **Clip library.** Pick a folder and browse your clips as a grid with thumbnails, duration, size, and date, plus search.
- **Player and manual trim.** Preview a clip, set start and end points with sliders, and save a trimmed copy. The original is never overwritten.
- **Auto Cut.** POTG scores the audio loudness and on-screen motion across the clip, finds the peak, and saves a trimmed copy around it with adjustable time before and after.
- **Settings.** Capture quality presets (Low, Medium, High, Custom) with resolution, frame rate, bitrate, and replay length, plus a live memory estimate.
- **Hotkeys.** Separate, rebindable shortcuts for saving a clip and for opening the overlay, with conflict warnings.
- **Overlay.** A small always-on-top window, opened by hotkey, with the same quality settings. It can be turned off completely in Settings.

## Coming next

- Audio from a single game instead of all system sound
- Captions and 9:16 vertical export for TikTok and Shorts
- Capture of a single game window

## How Auto Cut works

1. FFmpeg extracts the audio, and its loudness is measured over short time slices.
2. FFmpeg also supplies small grayscale frames, and the change between them measures on-screen motion.
3. The two signals are combined into one highlight score over time.
4. The highest peak is found, the window grows outward while the score stays high, and pre-roll and post-roll are added.
5. FFmpeg writes the trimmed clip.

All of this runs inside POTG, so nothing else needs to be installed.

## Tech stack

| Part | Technology |
|---|---|
| App shell | Tauri 2 |
| Interface | Svelte, SvelteKit, TypeScript |
| Backend | Rust |
| Audio capture | WASAPI through the cpal library |
| Video processing and encoding | FFmpeg |

## Getting started

### Requirements

- Node.js (LTS)
- Rust, installed from rustup.rs
- FFmpeg on your PATH (the Windows installer already includes it)
- Windows: Visual Studio C++ Build Tools. macOS: Xcode Command Line Tools (`xcode-select --install`)

### Run it

```
git clone https://github.com/YOUR-USERNAME/potg.git
cd potg
npm install
npm run tauri dev
```

The first run compiles the Rust code and takes a few minutes. On macOS, recording uses a test pattern and no system sound, because real screen and system audio capture are Windows-only for now.

### Environment override

For development you can point POTG at a specific FFmpeg with `POTG_FFMPEG`.

## Default hotkeys

| Action | Shortcut |
|---|---|
| Save a clip | Ctrl + Shift + F9 |
| Open or close the overlay | Ctrl + Shift + F10 |

Both can be changed in Settings. On a Mac, function keys may need the fn key held.

## Project structure

```
src/             Svelte interface (library, editor, settings, overlay)
src-tauri/       Rust backend (recording, audio, Auto Cut, settings, hotkeys, overlay)
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

- Recording captures the whole main screen, not a single game window.
- Sound capture records everything playing on your computer, including music and chat apps.
- The overlay is a normal window, so it will not appear over games in exclusive fullscreen. Borderless windowed mode is the target.
- The installers are unsigned, so Windows SmartScreen shows a warning. Choose More info, then Run anyway.

## Third-party software

The Windows installer includes an LGPL build of FFmpeg. See THIRD-PARTY-NOTICES.md for details.

## Contributing

Commit messages follow Conventional Commits, for example `feat(library): add manual trim` or `fix(capture): handle black frames`.

## License

To be decided.
