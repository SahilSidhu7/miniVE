<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { PortMap } from "./types";

  let { ports }: { ports: PortMap[] } = $props();
  let active = $state<number | undefined>();
  let visible = $state(false);
  let nonce = $state(0);
  let height = $state(320);

  // The iframe shows `url`; `urlInput` is the editable address bar, committed
  // on Enter or ↻. Switching ports resets both to that port's root.
  let url = $state("");
  let urlInput = $state("");

  $effect(() => {
    if (!ports.some((p) => p.host === active)) active = ports[0]?.host;
  });
  $effect(() => {
    url = `http://localhost:${active}`;
    urlInput = url;
  });

  function commitUrl() {
    let u = urlInput.trim();
    if (!u) return;
    if (u.startsWith("/")) u = `http://localhost:${active}${u}`;
    else if (!/^https?:\/\//i.test(u)) u = `http://${u}`;
    urlInput = u;
    url = u;
    nonce = nonce + 1;
  }

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
    <input
      class="url"
      bind:value={urlInput}
      onkeydown={(e) => e.key === "Enter" && commitUrl()}
      placeholder="http://localhost:8000/path"
      spellcheck="false"
    />
    <button onclick={commitUrl}>↻</button>
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
  .url { flex: 1; min-width: 8rem; background: #111; color: #ddd; border: 1px solid #333; border-radius: 4px; padding: 0.3em 0.5em; font-size: 0.85rem; }
  iframe { width: 100%; border: none; background: #fff; }
</style>
