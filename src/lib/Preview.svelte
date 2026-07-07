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

  const url = $derived(`http://localhost:${active}`);
</script>

<div class="preview" class:expanded={visible}>
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
      <iframe src={url} title="port preview"></iframe>
    {/key}
  {/if}
</div>

<style>
  .preview { border-top: 1px solid #333; }
  .bar { display: flex; gap: 0.5rem; padding: 0.25rem 0.5rem; align-items: center; }
  iframe { width: 100%; height: 40vh; border: none; background: #fff; }
</style>
