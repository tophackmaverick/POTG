<script lang="ts">
  import "./theme.css";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listClips, type ClipInfo } from "../api";
  import ClipCard from "./ClipCard.svelte";
  import EditorPanel from "./EditorPanel.svelte";
  import SettingsPanel from "./SettingsPanel.svelte";
  import { fileName } from "./format";
  import { settings, loadSettings, saveSettings, watchSettings } from "./settings";
  import {
    recorderStatus,
    refreshStatus,
    startRecording,
    stopRecording,
    defaultClipsFolder,
  } from "./recorder";

  let view: "library" | "settings" = "library";
  let folder = "";
  let defaultFolder = "";
  let clips: ClipInfo[] = [];
  let selected: ClipInfo | null = null;
  let query = "";
  let loading = false;
  let error = "";
  let toast = "";
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  let recBusy = false;

  $: folder = $settings ? $settings.clips_folder || defaultFolder : "";
  $: if (folder) load();
  $: needle = query.trim().toLowerCase();
  $: visible = clips.filter((c) => c.name.toLowerCase().includes(needle));

  function showToast(text: string, ms = 3500) {
    toast = text;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), ms);
  }

  async function load() {
    if (!folder) return;
    loading = true;
    error = "";
    try {
      clips = await listClips(folder);
      const current = selected;
      if (current && !clips.some((c) => c.path === current.path)) selected = null;
    } catch (e) {
      error = String(e);
      clips = [];
    } finally {
      loading = false;
    }
  }

  async function chooseFolder() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir !== "string" || !$settings) return;
    try {
      await saveSettings({ ...$settings, clips_folder: dir });
      selected = null;
    } catch (e) {
      showToast(String(e), 6000);
    }
  }

  async function toggleRecording() {
    recBusy = true;
    try {
      if ($recorderStatus.recording) await stopRecording();
      else await startRecording();
    } catch (e) {
      showToast(String(e), 8000);
    } finally {
      recBusy = false;
    }
  }

  function select(clip: ClipInfo) {
    selected = clip;
  }

  function toggleView() {
    view = view === "settings" ? "library" : "settings";
  }

  onMount(() => {
    let alive = true;
    const stops: Array<() => void> = [];
    const track = (pending: Promise<() => void>) =>
      pending.then((stop) => (alive ? stops.push(stop) : stop()));

    loadSettings().catch((e) => (error = String(e)));
    defaultClipsFolder()
      .then((f) => (defaultFolder = f))
      .catch((e) => (error = String(e)));
    refreshStatus().catch(() => {});

    track(watchSettings());
    track(
      listen<string>("clip-saved", (e) => {
        showToast(`Clip saved: ${fileName(e.payload)}`);
        load();
      }),
    );
    track(listen<string>("clip-failed", (e) => showToast(e.payload, 6000)));
    track(listen("recorder-changed", () => refreshStatus().catch(() => {})));

    const timer = setInterval(() => refreshStatus().catch(() => {}), 2000);

    return () => {
      alive = false;
      stops.forEach((stop) => stop());
      clearInterval(timer);
      clearTimeout(toastTimer);
    };
  });
</script>

<div class="app">
  <header class="topbar gloss">
    <div class="brand">
      <span class="logo">POTG</span>
      <span class="tagline">Your best moments, already edited.</span>
    </div>
    <div class="tools">
      {#if $recorderStatus.recording}
        <span class="rec-pill"><span class="dot"></span> REC{#if $recorderStatus.audio.includes("microphone")} · mic on{/if} · {$recorderStatus.buffered_seconds}s buffered</span>
      {/if}
      <button class="btn" class:rec-on={$recorderStatus.recording} on:click={toggleRecording} disabled={recBusy}>
        {#if recBusy}
          Please wait...
        {:else if $recorderStatus.recording}
          Stop recording
        {:else}
          Start recording
        {/if}
      </button>
      {#if view === "library"}
        <input class="search" type="search" placeholder="Search clips" bind:value={query} />
        <button class="btn" on:click={load} disabled={!folder || loading}>Refresh</button>
        <button class="btn" on:click={chooseFolder}>Change folder</button>
      {/if}
      <button class="btn btn-red" on:click={toggleView}>
        {view === "settings" ? "Back to clips" : "Settings"}
      </button>
    </div>
  </header>

  {#if view === "library"}
    <main class="body">
      <section class="library gloss">
        {#if !folder}
          <div class="empty"><span class="spinner"></span></div>
        {:else}
          <p class="path">{folder}</p>
          {#if error}
            <p class="error">{error}</p>
          {:else if loading && clips.length === 0}
            <div class="empty"><span class="spinner"></span></div>
          {:else if visible.length === 0}
            <div class="empty">
              <h2>No videos found</h2>
              <p>
                {clips.length === 0
                  ? "Start recording and press your clip hotkey, or add mp4, mkv, mov, or webm files to this folder."
                  : "No clips match your search."}
              </p>
            </div>
          {:else}
            <div class="grid">
              {#each visible as clip (clip.path)}
                <ClipCard {clip} selected={selected?.path === clip.path} onSelect={select} />
              {/each}
            </div>
          {/if}
        {/if}
      </section>

      <aside class="editor">
        {#key selected?.path}
          <EditorPanel clip={selected} onChanged={load} />
        {/key}
      </aside>
    </main>
  {:else}
    <main class="settings-view gloss">
      <SettingsPanel mode="full" />
    </main>
  {/if}

  {#if toast}
    <div class="toast">{toast}</div>
  {/if}
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    gap: 16px;
    height: 100vh;
    padding: 16px;
  }

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 18px;
    border-radius: var(--radius);
  }

  .brand {
    display: flex;
    align-items: baseline;
    gap: 14px;
    min-width: 0;
  }

  .logo {
    font-size: 26px;
    font-weight: 900;
    letter-spacing: 0.14em;
    background: linear-gradient(180deg, #ff8a94 0%, #e11d2e 50%, #8f0a16 100%);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    filter: drop-shadow(0 0 10px rgba(225, 29, 46, 0.45));
  }

  .tagline {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .search {
    width: 180px;
  }

  .rec-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 7px 12px;
    font-size: 12px;
    font-weight: 700;
    border-radius: 999px;
    border: 1px solid rgba(255, 91, 105, 0.6);
    background: rgba(225, 29, 46, 0.14);
    white-space: nowrap;
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: #ff3b4d;
    box-shadow: 0 0 10px #ff3b4d;
    animation: pulse 1.4s ease-in-out infinite;
  }

  .rec-on {
    border-color: var(--red-hi);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.14),
      0 0 14px var(--red-glow);
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }

  .body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 420px;
    gap: 16px;
    flex: 1;
    min-height: 0;
  }

  .library {
    min-height: 0;
    overflow-y: auto;
    padding: 16px;
    border-radius: var(--radius);
  }

  .editor {
    min-height: 0;
    overflow-y: auto;
    border-radius: var(--radius);
  }

  .settings-view {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 20px;
    border-radius: var(--radius);
  }

  .path {
    margin: 0 0 12px;
    font-size: 12px;
    color: var(--muted);
    word-break: break-all;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 14px;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    min-height: 260px;
    padding: 0 24px;
    text-align: center;
    color: var(--muted);
  }

  .empty h2 {
    margin: 0;
    font-size: 20px;
    color: var(--text);
  }

  .empty p {
    margin: 0;
    max-width: 420px;
    font-size: 13px;
  }

  .error {
    margin: 0;
    font-size: 13px;
    color: #ff8a94;
  }

  .toast {
    position: fixed;
    left: 50%;
    bottom: 28px;
    max-width: 80vw;
    transform: translateX(-50%);
    padding: 12px 20px;
    font-size: 13px;
    font-weight: 600;
    border-radius: 12px;
    border: 1px solid var(--red-hi);
    background: linear-gradient(180deg, #2a0b0f, #12060a);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.12),
      0 10px 30px var(--red-glow);
    word-break: break-word;
  }
</style>
