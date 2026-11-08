import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writable } from "svelte/store";
import type { AppSettings } from "./quality";

export const settings = writable<AppSettings | null>(null);

export async function loadSettings(): Promise<AppSettings> {
  const loaded = await invoke<AppSettings>("get_settings");
  settings.set(loaded);
  return loaded;
}

export async function saveSettings(next: AppSettings): Promise<AppSettings> {
  const saved = await invoke<AppSettings>("save_settings", { settings: next });
  settings.set(saved);
  return saved;
}

export async function resetSettings(): Promise<AppSettings> {
  const reset = await invoke<AppSettings>("reset_settings");
  settings.set(reset);
  return reset;
}

export function watchSettings(): Promise<() => void> {
  return listen<AppSettings>("settings-changed", (event) => settings.set(event.payload));
}

export const closeOverlay = () => invoke<void>("close_overlay");
