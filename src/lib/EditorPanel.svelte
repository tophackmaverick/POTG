<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { autoCut, trimClip, type ClipInfo } from "../api";
  import { fileName, formatClock } from "./format";

  export let clip: ClipInfo | null;
  export let onChanged: () => void;

  type SliderEvent = Event & { currentTarget: EventTarget & HTMLInputElement };

  let video: HTMLVideoElement;
  let duration = 0;
  let current = 0;
  let start = 0;
  let end = 0;
  let precise = false;
  let pre = 6;
  let post = 4;
  let busy: "" | "trim" | "auto" = "";
  let message = "";
  let error = "";
  let confidence: number | null = null;
  let previewing = false;

  $: src = clip ? convertFileSrc(clip.path) : "";
  $: startPct = duration ? (start / duration) * 100 : 0;
  $: endPct = duration ? (end / duration) * 100 : 100;
  $: playheadPct = duration ? (current / duration) * 100 : 0;
  $: valid = end - start >= 0.5;
  $: confidenceLabel =
    confidence === null
      ? ""
      : confidence >= 0.7
        ? "Clear highlight found"
        : confidence >= 0.4
          ? "Probable highlight, check the cut"
          : "Weak highlight, adjust the sliders if needed";

  function onLoaded() {
    duration = video.duration;
    start = 0;
    end = duration;
  }

  function onTime() {
    current = video.currentTime;
    if (previewing && current >= end) {
      video.pause();
      previewing = false;
    }
  }

  function moveStart(e: SliderEvent) {
    start = Math.max(0, Math.min(e.currentTarget.valueAsNumber, end - 0.5));
    e.currentTarget.value = String(start);
  }

  function moveEnd(e: SliderEvent) {
    end = Math.min(duration, Math.max(e.currentTarget.valueAsNumber, start + 0.5));
    e.currentTarget.value = String(end);
  }

  function setStartHere() {
    start = Math.max(0, Math.min(current, end - 0.5));
  }

  function setEndHere() {
    end = Math.min(duration, Math.max(current, start + 0.5));
  }

  function preview() {
    video.currentTime = start;
    previewing = true;
    video.play();
  }

  async function runTrim() {
    if (!clip) return;
    const path = clip.path;
    busy = "trim";
    error = "";
    message = "";
    try {
      const out = await trimClip(path, start, end, precise);
      message = `Saved ${fileName(out)} next to the original`;
      onChanged();
    } catch (e) {
      error = String(e);
    } finally {
      busy = "";
    }
  }

  async function runAutoCut() {
    if (!clip) return;
    const path = clip.path;
    busy = "auto";
    error = "";
    message = "";
    confidence = null;
    try {
      const result = await autoCut(path, pre, post);
      start = result.start;
      end = result.end;
      confidence = result.confidence;
      message = `Saved ${fileName(result.output)} next to the original`;
      onChanged();
      preview();
    } catch (e) {
      error = String(e);
    } finally {
      busy = "";
    }
  }
</script>

<div class="panel gloss">
  {#if !clip}
    <div class="placeholder">
      <h3>Select a clip</h3>
      <p>Pick one from the library to preview it, trim it, or let Auto Cut finish it.</p>
    </div>
  {:else}
    <h3 class="title" title={clip.name}>{clip.name}</h3>

    <div class="player">
      <video
        bind:this={video}
        {src}
        controls
        preload="metadata"
        on:loadedmetadata={onLoaded}
        on:timeupdate={onTime}
        on:pause={() => (previewing = false)}
      >
        <track kind="captions" />
      </video>
    </div>

    <div class="timeline">
      <div class="track">
        <div class="range" style="left: {startPct}%; width: {Math.max(0, endPct - startPct)}%"></div>
        <div class="playhead" style="left: {playheadPct}%"></div>
      </div>
      <div class="times">
        <span>{formatClock(start)}</span>
        <span>{formatClock(Math.max(0, end - start))} selected</span>
        <span>{formatClock(end)}</span>
      </div>
    </div>

    <label class="slider">
      <span>Start</span>
      <input type="range" min="0" max={duration} step="0.1" value={start} on:input={moveStart} />
    </label>
    <label class="slider">
      <span>End</span>
      <input type="range" min="0" max={duration} step="0.1" value={end} on:input={moveEnd} />
    </label>

    <div class="row">
      <button class="btn" on:click={setStartHere}>Set start here</button>
      <button class="btn" on:click={setEndHere}>Set end here</button>
      <button class="btn" on:click={preview}>Preview</button>
    </div>

    <label class="check">
      <input type="checkbox" bind:checked={precise} />
      <span>Frame-accurate trim (slower)</span>
    </label>

    <button class="btn btn-large btn-wide" on:click={runTrim} disabled={busy !== "" || !valid}>
      {busy === "trim" ? "Trimming..." : "Trim selection"}
    </button>

    <div class="auto">
      <div class="auto-head">
        <h4>Auto Cut</h4>
        <p>POTG finds the peak moment and cuts around it.</p>
      </div>
      <div class="pads">
        <label>Before <input type="number" min="0" max="30" step="1" bind:value={pre} /> s</label>
        <label>After <input type="number" min="0" max="30" step="1" bind:value={post} /> s</label>
      </div>
      <button class="btn btn-red btn-large btn-wide" on:click={runAutoCut} disabled={busy !== ""}>
        {#if busy === "auto"}
          <span class="spinner"></span> Analyzing...
        {:else}
          Auto Cut
        {/if}
      </button>
    </div>

    {#if confidenceLabel}
      <p class="note">{confidenceLabel}</p>
    {/if}
    {#if message}
      <p class="ok">{message}</p>
    {/if}
    {#if error}
      <p class="err">{error}</p>
    {/if}
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 100%;
    padding: 18px;
    border-radius: var(--radius);
  }

  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    flex: 1;
    text-align: center;
    color: var(--muted);
  }

  .placeholder h3 {
    margin: 0;
    color: var(--text);
  }

  .placeholder p {
    margin: 0;
    max-width: 260px;
    font-size: 13px;
  }

  .title {
    margin: 0;
    font-size: 15px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .player {
    overflow: hidden;
    border-radius: 12px;
    border: 1px solid var(--border-strong);
    background: #000;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.6);
  }

  video {
    display: block;
    width: 100%;
    aspect-ratio: 16 / 9;
    background: #000;
  }

  .timeline {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .track {
    position: relative;
    height: 12px;
    border-radius: 8px;
    border: 1px solid var(--border-strong);
    background: linear-gradient(180deg, #050506, #131316);
    overflow: hidden;
  }

  .range {
    position: absolute;
    top: 0;
    bottom: 0;
    background: linear-gradient(180deg, #ff6a77 0%, #e11d2e 50%, #a30f1c 52%, #cf1629 100%);
    box-shadow: 0 0 12px var(--red-glow);
  }

  .playhead {
    position: absolute;
    top: -1px;
    bottom: -1px;
    width: 2px;
    background: #fff;
    box-shadow: 0 0 6px rgba(255, 255, 255, 0.8);
  }

  .times {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .slider {
    display: grid;
    grid-template-columns: 44px 1fr;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
  }

  .row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .row .btn {
    padding: 9px 6px;
    font-size: 12px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
  }

  .auto {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border-radius: 12px;
    border: 1px solid rgba(225, 29, 46, 0.4);
    background: linear-gradient(180deg, rgba(225, 29, 46, 0.12), rgba(225, 29, 46, 0.02));
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
  }

  .auto-head h4 {
    margin: 0 0 2px;
    font-size: 15px;
  }

  .auto-head p {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  .pads {
    display: flex;
    gap: 14px;
    font-size: 12px;
    color: var(--muted);
  }

  .pads input {
    width: 60px;
    padding: 6px 8px;
  }

  .note {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  .ok {
    margin: 0;
    font-size: 12px;
    color: #9ee6a8;
    word-break: break-all;
  }

  .err {
    margin: 0;
    font-size: 12px;
    color: #ff8a94;
    word-break: break-word;
  }
</style>
