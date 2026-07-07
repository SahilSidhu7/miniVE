<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, Channel } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";

  type Node = { name: string; is_dir: boolean; path: string; children?: Node[]; open?: boolean };
  let { env }: { env: string } = $props();
  let roots: Node[] = $state([]);
  let status = $state("");

  async function load(path: string): Promise<Node[]> {
    const items = await invoke<{ name: string; is_dir: boolean }[]>("list_files", { name: env, path });
    return items.map((i) => ({ ...i, path: path ? `${path}/${i.name}` : i.name }));
  }

  async function refresh() {
    try {
      roots = await load("");
    } catch (e) {
      status = String(e);
    }
  }

  async function toggle(node: Node) {
    if (!node.is_dir) return;
    node.open = !node.open;
    if (node.open && !node.children) {
      try {
        node.children = await load(node.path);
      } catch (e) {
        status = String(e);
        node.open = false;
      }
    }
    roots = [...roots];
  }

  async function upload() {
    const picked = await open({ multiple: true });
    if (!picked) return;
    status = "Uploading…";
    try {
      await invoke("upload_paths", { name: env, paths: Array.isArray(picked) ? picked : [picked] });
      status = "";
      await refresh();
    } catch (e) { status = String(e); }
  }

  async function clone() {
    const url = prompt("Git repository URL:");
    if (!url) return;
    status = "Cloning…";
    const out = new Channel<string>();
    out.onmessage = (l) => (status = l.slice(0, 120));
    try {
      const code = await invoke<number>("clone_repo", { name: env, url, onOutput: out });
      status = code === 0 ? "" : `clone failed (exit ${code})`;
    } catch (e) {
      status = String(e);
    }
    await refresh();
  }

  onMount(refresh);
</script>

{#snippet tree(nodes: Node[])}
  <ul>
    {#each nodes as node (node.path)}
      <li>
        <button class="node" onclick={() => toggle(node)}>
          {node.is_dir ? (node.open ? "▾ 📁" : "▸ 📁") : "📄"} {node.name}
        </button>
        {#if node.open && node.children}{@render tree(node.children)}{/if}
      </li>
    {/each}
  </ul>
{/snippet}

<div class="filetree">
  <div class="toolbar">
    <button onclick={upload}>Upload</button>
    <button onclick={clone}>Clone</button>
    <button onclick={refresh}>↻</button>
  </div>
  {#if status}<p class="status">{status}</p>{/if}
  {@render tree(roots)}
</div>

<style>
  .filetree { padding: 0.5rem; font-size: 0.85rem; }
  .toolbar { display: flex; gap: 0.25rem; margin-bottom: 0.5rem; }
  ul { list-style: none; padding-left: 0.9rem; margin: 0; }
  .node { background: none; border: none; color: inherit; cursor: pointer; padding: 1px 0; }
  .status { color: #999; font-size: 0.75rem; word-break: break-all; }
</style>
