<script lang="ts">
  import "../../lib/theme.css";
  import SettingsPanel from "../../lib/SettingsPanel.svelte";
  import { closeOverlay } from "../../lib/settings";

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") closeOverlay();
  }
</script>

<svelte:window on:keydown={onKey} />

<div class="overlay gloss">
  <header data-tauri-drag-region>
    <div class="title" data-tauri-drag-region>
      <span class="logo" data-tauri-drag-region>POTG</span>
      <span class="sub" data-tauri-drag-region>Overlay</span>
    </div>
    <button class="btn" on:click={closeOverlay}>Close</button>
  </header>
  <div class="content">
    <SettingsPanel mode="overlay" />
  </div>
</div>

<style>
  .overlay {
    display: flex;
    flex-direction: column;
    height: 100vh;
    border-radius: 0;
    border-color: var(--red);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-strong);
    user-select: none;
  }

  .title {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .logo {
    font-size: 20px;
    font-weight: 900;
    letter-spacing: 0.14em;
    background: linear-gradient(180deg, #ff8a94 0%, #e11d2e 50%, #8f0a16 100%);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }

  .sub {
    font-size: 12px;
    color: var(--muted);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 14px;
  }
</style>
