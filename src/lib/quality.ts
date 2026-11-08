export type Preset = "low" | "medium" | "high" | "custom";
export type Resolution = "720p" | "1080p" | "1440p" | "native";

export interface QualitySettings {
  preset: Preset;
  resolution: Resolution;
  fps: number;
  bitrate_mbps: number;
  replay_seconds: number;
}

export interface AppSettings {
  quality: QualitySettings;
  overlay_enabled: boolean;
  clip_hotkey: string;
  overlay_hotkey: string;
  clips_folder: string;
}

export const PRESETS: Record<
  Exclude<Preset, "custom">,
  Pick<QualitySettings, "resolution" | "fps" | "bitrate_mbps">
> = {
  low: { resolution: "720p", fps: 30, bitrate_mbps: 6 },
  medium: { resolution: "1080p", fps: 60, bitrate_mbps: 20 },
  high: { resolution: "1440p", fps: 60, bitrate_mbps: 40 },
};

export const PRESET_LIST: { value: Preset; label: string }[] = [
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "custom", label: "Custom" },
];

export const RESOLUTIONS: { value: Resolution; label: string }[] = [
  { value: "720p", label: "720p" },
  { value: "1080p", label: "1080p" },
  { value: "1440p", label: "1440p" },
  { value: "native", label: "Native (screen size)" },
];

export const DEFAULT_SETTINGS: AppSettings = {
  quality: {
    preset: "medium",
    resolution: "1080p",
    fps: 60,
    bitrate_mbps: 20,
    replay_seconds: 60,
  },
  overlay_enabled: true,
  clip_hotkey: "Ctrl+Shift+F9",
  overlay_hotkey: "Ctrl+Shift+F10",
  clips_folder: "",
};

export function estimateMb(q: Pick<QualitySettings, "bitrate_mbps" | "replay_seconds">): number {
  return Math.round((q.bitrate_mbps * q.replay_seconds) / 8);
}
