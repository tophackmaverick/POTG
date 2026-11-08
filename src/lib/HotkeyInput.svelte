<script lang="ts">
  import { buildHotkey, prettyHotkey } from "./hotkey";

  export let value: string;
  export let disabled = false;

  let listening = false;
  let hint = "";

  function start() {
    if (disabled) return;
    listening = true;
    hint = "";
  }

  function stop() {
    listening = false;
  }

  function onKeyDown(e: KeyboardEvent) {
    if (!listening) return;
    e.preventDefault();
    if (e.code === "Escape") {
      stop();
      return;
    }
    const result = buildHotkey(e);
    if (result.value) {
      value = result.value;
      hint = "";
      stop();
    } else {
      hint = result.hint ?? "";
    }
  }
</script>

<div class="wrap">
  <button
    type="button"
    class="key"
    class:listening
    {disabled}
    on:click={start}
    on:keydown={onKeyDown}
    on:blur={stop}
  >
    {listening ? "Press the new shortcut..." : prettyHotkey(value)}
  </button>
  {#if hint}
    <span class="hint">{hint}</span>
  {/if}
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .key {
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text);
    padding: 10px 14px;
    text-align: center;
    cursor: pointer;
    border-radius: 10px;
    border: 1px solid var(--border-strong);
    background: linear-gradient(180deg, #08080a, #131316);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
  }

  .key:hover:not(:disabled) {
    border-color: var(--red);
  }

  .key.listening {
    border-color: var(--red-hi);
    box-shadow:
      0 0 0 3px rgba(225, 29, 46, 0.25),
      0 0 18px var(--red-glow);
  }

  .key:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .hint {
    font-size: 11px;
    color: #ff8a94;
  }
</style>
