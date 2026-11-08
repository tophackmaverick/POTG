<script lang="ts">
  import "./theme.css";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listClips, type ClipInfo } from "../api";
  import ClipCard from "./ClipCard.svelte";
  import EditorPanel from "./EditorPanel.svelte";
  import SettingsPanel from "./SettingsPanel.svelte";

  const FOLDER_KEY = "potg.folder";

  let view: "library" | "settings" = "library";
  let folder = "";
  let clips: ClipInfo[] = [];
  let selected: ClipInfo | null = null;
  let query = "";
  let loading = false;
  let error = "";
  let toast = "";
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  $: needle = query.trim().toLowerCase();
  $: visible = clips.filter((c) => c.name.toLowerCase().includes(needle));

  function showToast(text: string) {
    toast = text;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), 3500);
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
    if (typeof dir !== "string") return;
    folder = dir;
    localStorage.setItem(FOLDER_KEY, dir);
    selected = null;
    await load();
  }

  function select(clip: ClipInfo) {
    selected = clip;
  }

  function toggleView() {
    view = view === "settings" ? "library" : "settings";
  }

  onMount(() => {
    folder = localStorage.getItem(FOLDER_KEY) ?? "";
    load();
    let stop: (() => void) | undefined;
    listen("clip-requested", () =>
      showToast("Clip hotkey pressed. Saving real clips arrives with native capture."),
    ).then((unlisten) => (stop = unlisten));
    return () => {
      stop?.();
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
      {#if view === "library"}
        <input class="search" type="search" placeholder="Search clips" bind:value={query} />
        <button class="btn" on:click={load} disabled={!folder || loading}>Refresh</button>
        <button class="btn" on:click={chooseFolder}>{folder ? "Change folder" : "Choose folder"}</button>
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
          <div class="empty">
            <h2>Welcome to POTG</h2>
            <p>Choose the folder where your gameplay clips are saved.</p>
            <button class="btn btn-red btn-large" on:click={chooseFolder}>Choose clips folder</button>
          </div>
        {:else}
          <p class="path">{folder}</p>
          {#if error}
            <p class="error">{error}</p>
          {:else if loading && clips.length === 0}
            <div class="empty"><span class="spinner"></span></div>
          {:else if visible.length === 0}
            <div class="empty">
              <h2>No videos found</h2>
              <p>{clips.length === 0 ? "This folder has no mp4, mkv, mov, or webm files." : "No clips match your search."}</p>
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
    padding: 12px 18px;
    border-radius: var(--radius);
  }

  .brand {
    display: flex;
    align-items: baseline;
    gap: 14px;
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
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .search {
    width: 220px;
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
  }
</style>
