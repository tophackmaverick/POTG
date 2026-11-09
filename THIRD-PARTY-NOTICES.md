# Third-party notices

POTG includes or uses the following third-party software.

## FFmpeg

POTG records, trims, and processes video by running FFmpeg as a separate program. The Windows installer includes an unmodified LGPL build of FFmpeg, saved as `ffmpeg.exe` in the same folder as POTG. POTG does not link FFmpeg into its own code.

- Project: https://ffmpeg.org
- License: GNU Lesser General Public License, version 2.1 or later (https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html)
- Build used: the LGPL static build from https://github.com/BtbN/FFmpeg-Builds, which does not include GPL-only components such as x264
- Source code: available from https://ffmpeg.org/download.html and from the build scripts at https://github.com/BtbN/FFmpeg-Builds

You may replace `ffmpeg.exe` in the POTG installation folder with another compatible FFmpeg build.

## Other components

POTG is built with Tauri, Rust, Svelte, and the cpal audio library (Apache License 2.0). Each is used under its own open-source license.
