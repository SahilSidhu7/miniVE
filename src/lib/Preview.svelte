<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { PortMap } from "./types";

  let { ports }: { ports: PortMap[] } = $props();
  let active = $state<number | undefined>();
  $effect(() => {
    if (!ports.some((p) => p.host === active)) active = ports[0]?.host;
  });
  let visible = $state(false);
  let nonce = $state(0);
  let height = $state(320);

  const url = $derived(`http://localhost:${active}`);

  function dragHeight(e: PointerEvent) {
    e.preventDefault();
    const startY = e.clientY;
    const startH = height;
    const move = (ev: PointerEvent) => {
      height = Math.min(window.innerHeight - 160, Math.max(120, startH + (startY - ev.clientY)));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }
</script>

<div class="preview" class:expanded={visible}>
  {#if visible}
    <div
      class="hdrag"
      role="separator"
      aria-orientation="horizontal"
      aria-label="Resize preview"
      onpointerdown={dragHeight}
    ></div>
  {/if}
  <div class="bar">
    <button onclick={() => (visible = !visible)}>{visible ? "▾" : "▸"} Preview</button>
    <select bind:value={active}>
      {#each ports as p (p.host)}<option value={p.host}>localhost:{p.host} → {p.container}</option>{/each}
    </select>
    <button onclick={() => (nonce = nonce + 1)}>↻</button>
    <button onclick={() => openUrl(url)}>Open in browser</button>
  </div>
  {#if visible}
    {#key `${url}-${nonce}`}
      <iframe src={url} title="port preview" style:height={`${height}px`}></iframe>
    {/key}
  {/if}
</div>

<style>
  .preview { border-top: 1px solid #333; }
  .hdrag { height: 5px; cursor: row-resize; margin-bottom: -3px; }
  .hdrag:hover { background: #646cff44; }
  .bar { display: flex; gap: 0.5rem; padding: 0.25rem 0.5rem; align-items: center; }
  iframe { width: 100%; border: none; background: #fff; }
</style>
