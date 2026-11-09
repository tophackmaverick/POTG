<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { settings, loadSettings, saveSettings, resetSettings, watchSettings } from "./settings";
  import {
    DEFAULT_SETTINGS,
    PRESETS,
    PRESET_LIST,
    RESOLUTIONS,
    estimateMb,
    type AppSettings,
    type Preset,
  } from "./quality";
  import { prettyHotkey } from "./hotkey";
  import HotkeyInput from "./HotkeyInput.svelte";
  import { recorderStatus } from "./recorder";

  export let mode: "full" | "overlay" = "full";

  let draft: AppSettings = structuredClone(DEFAULT_SETTINGS);
  let saved: AppSettings = structuredClone(DEFAULT_SETTINGS);
  let ready = false;
  let busy = false;
  let error = "";
  let message = "";
  let unlisten: (() => void) | null = null;

  $: dirty = JSON.stringify(draft) !== JSON.stringify(saved);
  $: diagnostics = [
    `recording: ${$recorderStatus.recording}`,
    `encoder: ${$recorderStatus.encoder || "-"}`,
    `source: ${$recorderStatus.source || "-"}`,
    `audio: ${$recorderStatus.audio || "-"}`,
    `buffered seconds: ${$recorderStatus.buffered_seconds}`,
    `last error: ${$recorderStatus.last_error || "-"}`,
    `command: ${$recorderStatus.command || "-"}`,
    "log:",
    ...$recorderStatus.log,
  ].join("\n");
  $: sync($settings);

  function sync(next: AppSettings | null) {
    if (!next) return;
    const wasDirty = JSON.stringify(draft) !== JSON.stringify(saved);
    saved = structuredClone(next);
    if (!wasDirty || !ready) draft = structuredClone(next);
    ready = true;
  }

  function pickPreset(preset: Preset) {
    if (preset === "custom") {
      draft.quality.preset = "custom";
    } else {
      draft.quality = { ...draft.quality, preset, ...PRESETS[preset] };
    }
  }

  async function chooseClipsFolder() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") draft.clips_folder = dir;
  }

  async function copyDiagnostics() {
    try {
      await navigator.clipboard.writeText(diagnostics);
      message = "Diagnostics copied.";
    } catch {
      error = "Could not copy. Select the text and copy it by hand.";
    }
  }

  function markCustom() {
    draft.quality.preset = "custom";
  }

  async function save() {
    busy = true;
    error = "";
    message = "";
    try {
      await saveSettings(draft);
      message = "Settings saved. Quality changes apply from the next recording.";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function reset() {
    busy = true;
    error = "";
    message = "";
    try {
      await resetSettings();
      draft = structuredClone(DEFAULT_SETTINGS);
      message = "Settings restored to defaults.";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(async () => {
    try {
      await loadSettings();
      unlisten = await watchSettings();
    } catch (e) {
      error = String(e);
    }
  });

  onDestroy(() => unlisten?.());
</script>

<div class="settings" class:compact={mode === "overlay"}>
  {#if !ready}
    <p class="muted">{error || "Loading settings..."}</p>
  {:else}
    <section class="group">
      <h3>Capture quality</h3>
      <div class="seg">
        {#each PRESET_LIST as p}
          <button
            type="button"
            class="seg-btn"
            class:active={draft.quality.preset === p.value}
            on:click={() => pickPreset(p.value)}
          >
            {p.label}
          </button>
        {/each}
      </div>

      <div class="two">
        <label class="field">
          <span>Resolution</span>
          <select bind:value={draft.quality.resolution} on:change={markCustom}>
            {#each RESOLUTIONS as r}
              <option value={r.value}>{r.label}</option>
            {/each}
          </select>
        </label>
        <label class="field">
          <span>Frame rate</span>
          <select bind:value={draft.quality.fps} on:change={markCustom}>
            <option value={30}>30 FPS</option>
            <option value={60}>60 FPS</option>
          </select>
        </label>
      </div>

      <label class="field">
        <span>Bitrate <b>{draft.quality.bitrate_mbps} Mbps</b></span>
        <input type="range" min="2" max="100" step="1" bind:value={draft.quality.bitrate_mbps} on:input={markCustom} />
      </label>

      <label class="field">
        <span>Replay length <b>{draft.quality.replay_seconds} s</b></span>
        <input type="range" min="15" max="120" step="5" bind:value={draft.quality.replay_seconds} />
      </label>

      <p class="estimate">
        About {estimateMb(draft.quality)} MB for the replay buffer and for each saved clip. Audio adds a little more.
      </p>
    </section>

    {#if mode === "full"}
      <section class="group">
        <h3>Audio</h3>
        <label class="check">
          <input type="checkbox" bind:checked={draft.capture_system_audio} />
          <span>Record game and system sound</span>
        </label>
        <label class="check">
          <input type="checkbox" bind:checked={draft.capture_mic} />
          <span>Record my microphone (separate track)</span>
        </label>
        <label class="field">
          <span>Audio delay <b>{draft.audio_offset_ms} ms</b></span>
          <input type="range" min="-1000" max="2000" step="50" bind:value={draft.audio_offset_ms} />
        </label>
        <p class="muted">
          If the sound arrives before the picture, increase the delay. If it arrives after, lower it. Audio changes apply from the next recording. If the microphone is silent, check that Windows allows apps to use it in Settings, Privacy, Microphone.
        </p>
      </section>

      <section class="group">
        <h3>Clips folder</h3>
        <p class="path">{draft.clips_folder || "Default: your Videos folder, in a folder named POTG"}</p>
        <div class="two">
          <button type="button" class="btn" on:click={chooseClipsFolder}>Choose folder</button>
          <button type="button" class="btn" on:click={() => (draft.clips_folder = "")}>Use default</button>
        </div>
        <p class="muted">Saved clips go here, and the library shows this folder.</p>
      </section>

      <section class="group">
        <h3>Overlay</h3>
        <label class="check">
          <input type="checkbox" bind:checked={draft.overlay_enabled} />
          <span>Enable the in-game overlay</span>
        </label>
        <p class="muted">When off, the overlay window is closed and its hotkey is released.</p>
      </section>

      <section class="group">
        <h3>Hotkeys</h3>
        <div class="field">
          <span>Save a clip</span>
          <HotkeyInput bind:value={draft.clip_hotkey} />
        </div>
        <div class="field">
          <span>Open or close the overlay</span>
          <HotkeyInput bind:value={draft.overlay_hotkey} disabled={!draft.overlay_enabled} />
        </div>
        <p class="muted">
          Use at least one modifier key. If another app already uses a shortcut, POTG tells you when you save.
        </p>
      </section>
      <section class="group">
        <h3>Recording</h3>
        <p class="muted">
          {$recorderStatus.recording
            ? `Recording with ${$recorderStatus.encoder} from ${$recorderStatus.source}.`
            : "Not recording. Use Start recording at the top of the app."}
        </p>
        {#if $recorderStatus.last_error}
          <p class="err">{$recorderStatus.last_error}</p>
        {/if}
        <details>
          <summary>Diagnostics</summary>
          <pre>{diagnostics}</pre>
          <button type="button" class="btn" on:click={copyDiagnostics}>Copy diagnostics</button>
        </details>
      </section>
    {:else}
      <label class="check">
        <input type="checkbox" bind:checked={draft.capture_system_audio} />
        <span>Record game and system sound</span>
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={draft.capture_mic} />
        <span>Record my microphone</span>
      </label>
      <p class="muted">
        Save a clip: {prettyHotkey(draft.clip_hotkey)}. Close this overlay: {prettyHotkey(draft.overlay_hotkey)} or Esc.
      </p>
    {/if}

    <div class="actions">
      <button class="btn btn-red btn-large" on:click={save} disabled={busy || !dirty}>
        {busy ? "Saving..." : "Save settings"}
      </button>
      <button class="btn btn-large" on:click={reset} disabled={busy}>Reset to defaults</button>
    </div>

    {#if message}
      <p class="ok">{message}</p>
    {/if}
    {#if error}
      <p class="err">{error}</p>
    {/if}
  {/if}
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 18px;
    max-width: 640px;
    margin: 0 auto;
    padding: 4px;
  }

  .settings.compact {
    gap: 14px;
    max-width: none;
    margin: 0;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: linear-gradient(180deg, #131317, #08080a);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
  }

  h3 {
    margin: 0;
    font-size: 15px;
  }

  .seg {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
    padding: 4px;
    border-radius: 12px;
    border: 1px solid var(--border-strong);
    background: #050506;
  }

  .seg-btn {
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    color: var(--muted);
    padding: 8px 4px;
    cursor: pointer;
    border-radius: 9px;
    border: 1px solid transparent;
    background: transparent;
  }

  .seg-btn:hover {
    color: var(--text);
  }

  .seg-btn.active {
    color: #fff;
    border-color: var(--red-hi);
    background: linear-gradient(180deg, #ff6a77 0%, #e11d2e 48%, #a30f1c 52%, #cf1629 100%);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.5),
      0 4px 14px var(--red-glow);
  }

  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }

  .field b {
    float: right;
    color: var(--text);
  }

  select {
    font: inherit;
    font-size: 13px;
    color: var(--text);
    padding: 9px 10px;
    border-radius: 10px;
    border: 1px solid var(--border-strong);
    background: linear-gradient(180deg, #08080a, #111114);
    outline: none;
  }

  select:focus {
    border-color: var(--red);
  }

  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }

  .estimate {
    margin: 0;
    padding: 10px 12px;
    font-size: 12px;
    border-radius: 10px;
    border: 1px solid rgba(225, 29, 46, 0.35);
    background: rgba(225, 29, 46, 0.08);
  }

  .muted {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  .path {
    margin: 0;
    font-size: 12px;
    word-break: break-all;
  }

  details {
    display: flex;
    flex-direction: column;
    gap: 10px;
    font-size: 12px;
    color: var(--muted);
  }

  summary {
    cursor: pointer;
  }

  pre {
    margin: 10px 0;
    padding: 10px;
    max-height: 220px;
    overflow: auto;
    font-size: 11px;
    white-space: pre-wrap;
    word-break: break-all;
    border-radius: 8px;
    border: 1px solid var(--border-strong);
    background: #050506;
    color: var(--text);
  }

  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .ok {
    margin: 0;
    font-size: 12px;
    color: #9ee6a8;
  }

  .err {
    margin: 0;
    font-size: 12px;
    color: #ff8a94;
    word-break: break-word;
  }
</style>
