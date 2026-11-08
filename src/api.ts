import { invoke } from "@tauri-apps/api/core";

export interface ClipInfo {
  path: string;
  name: string;
  size_bytes: number;
  modified_unix: number;
}

export interface AutoCutResult {
  output: string;
  peak: number;
  start: number;
  end: number;
  confidence: number;
}

export const listClips = (folder: string) =>
  invoke<ClipInfo[]>("list_clips", { folder });

export const getDuration = (input: string) =>
  invoke<number>("get_duration", { input });

export const makeThumbnail = (input: string, atSeconds: number) =>
  invoke<string>("make_thumbnail", { input, atSeconds });

export const trimClip = (
  input: string,
  start: number,
  end: number,
  precise = false,
  output?: string,
) => invoke<string>("trim_clip", { input, start, end, precise, output });

export const autoCut = (input: string, pre = 6, post = 4) =>
  invoke<AutoCutResult>("auto_cut", { input, pre, post });
