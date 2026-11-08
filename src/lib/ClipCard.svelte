<script lang="ts">
  import { onMount } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { getDuration, makeThumbnail, type ClipInfo } from "../api";
  import { limited } from "./queue";
  import { formatDuration, formatSize, formatDate } from "./format";

  export let clip: ClipInfo;
  export let selected = false;
  export let onSelect: (clip: ClipInfo) => void;

  let thumb = "";
  let duration = 0;
  let failed = false;

  onMount(async () => {
    try {
      duration = await limited(() => getDuration(clip.path));
      const file = await limited(() => makeThumbnail(clip.path, Math.min(1, duration / 2)));
      thumb = convertFileSrc(file);
    } catch {
      failed = true;
    }
  });
</script>

<button class="card" class:selected on:click={() => onSelect(clip)} title={clip.name}>
  <div class="thumb">
    {#if thumb}
      <img src={thumb} alt="" draggable="false" />
    {:else if failed}
      <span class="fallback">No preview</span>
    {:else}
      <span class="spinner"></span>
    {/if}
    {#if duration}
      <span class="badge">{formatDuration(duration)}</span>
    {/if}
  </div>
  <div class="meta">
    <span class="name">{clip.name}</span>
    <span class="sub">{formatSize(clip.size_bytes)} · {formatDate(clip.modified_unix)}</span>
  </div>
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 0;
    overflow: hidden;
    text-align: left;
    font: inherit;
    color: inherit;
    cursor: pointer;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: linear-gradient(180deg, #1c1c21 0%, #0d0d10 55%, #070708 100%);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.08),
      0 6px 18px rgba(0, 0, 0, 0.5);
    transition: transform 0.12s, border-color 0.15s, box-shadow 0.15s;
  }

  .card:hover {
    transform: translateY(-2px);
    border-color: var(--border-strong);
  }

  .card.selected {
    border-color: var(--red-hi);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.12),
      0 0 0 1px var(--red),
      0 8px 26px var(--red-glow);
  }

  .thumb {
    position: relative;
    display: grid;
    place-items: center;
    aspect-ratio: 16 / 9;
    background: #000;
    overflow: hidden;
  }

  .thumb::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.1) 0%, rgba(255, 255, 255, 0) 35%);
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .fallback {
    font-size: 12px;
    color: var(--muted);
  }

  .badge {
    position: absolute;
    right: 8px;
    bottom: 8px;
    z-index: 1;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 700;
    border-radius: 6px;
    background: linear-gradient(180deg, #2a2a30, #09090b);
    border: 1px solid var(--border-strong);
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 12px 12px;
  }

  .name {
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub {
    font-size: 11px;
    color: var(--muted);
  }
</style>
