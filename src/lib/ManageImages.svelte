<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { isPinned, pinVersion, unpinVersion, loadPinned } from "./catalog";

  let { onclose }: { onclose: () => void } = $props();

  type CachedImage = { id: string; repoTag: string; sizeBytes: number; createdUnix: number };
  let images: CachedImage[] = $state([]);
  let error = $state("");
  let busyId = $state("");

  function formatSize(bytes: number): string {
    const mb = bytes / (1024 * 1024);
    return mb > 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(0)} MB`;
  }

  async function refresh() {
    try {
      images = await invoke<CachedImage[]>("list_cached_images");
      await loadPinned();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(id: string) {
    busyId = id;
    try {
      await invoke("remove_cached_image", { id });
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busyId = "";
    }
  }

  async function togglePin(repoTag: string) {
    if (isPinned(repoTag)) await unpinVersion(repoTag);
    else await pinVersion(repoTag);
    images = [...images]; // re-render pin state
  }

  onMount(refresh);
</script>

<div class="overlay">
  <div class="modal">
    <h2>Manage Images</h2>
    {#if error}<p class="error">{error}</p>{/if}
    <table>
      <thead><tr><th>Image</th><th>Size</th><th>Pin</th><th></th></tr></thead>
      <tbody>
        {#each images as img (img.id)}
          <tr>
            <td>{img.repoTag}</td>
            <td>{formatSize(img.sizeBytes)}</td>
            <td><button onclick={() => togglePin(img.repoTag)}>{isPinned(img.repoTag) ? "★" : "☆"}</button></td>
            <td><button disabled={busyId === img.id} onclick={() => remove(img.id)}>{busyId === img.id ? "…" : "Delete"}</button></td>
          </tr>
        {:else}
          <tr><td colspan="4">No cached images.</td></tr>
        {/each}
      </tbody>
    </table>
    <div class="actions">
      <button onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: grid; place-items: center; }
  .modal { background: #1e1e1e; border-radius: 8px; padding: 1.5rem; width: 36rem; max-height: 85vh; overflow-y: auto; display: grid; gap: 0.75rem; }
  table { width: 100%; border-collapse: collapse; }
  th, td { text-align: left; padding: 0.35rem 0.5rem; border-bottom: 1px solid #333; }
  .error { color: #f87171; }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
</style>
