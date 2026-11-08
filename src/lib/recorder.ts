import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";

export interface RecorderStatus {
  recording: boolean;
  encoder: string;
  source: string;
  buffered_seconds: number;
  last_error: string;
  command: string;
  log: string[];
}

export const EMPTY_STATUS: RecorderStatus = {
  recording: false,
  encoder: "",
  source: "",
  buffered_seconds: 0,
  last_error: "",
  command: "",
  log: [],
};

export const recorderStatus = writable<RecorderStatus>(EMPTY_STATUS);

export async function refreshStatus(): Promise<void> {
  recorderStatus.set(await invoke<RecorderStatus>("recorder_status"));
}

export async function startRecording(): Promise<void> {
  await invoke("start_recording");
  await refreshStatus();
}

export async function stopRecording(): Promise<void> {
  await invoke("stop_recording");
  await refreshStatus();
}

export const saveClipNow = () => invoke<string>("save_clip_now");

export const defaultClipsFolder = () => invoke<string>("default_clips_folder");
