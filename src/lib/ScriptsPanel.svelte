<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, Channel } from "@tauri-apps/api/core";
  import type { ScriptEntry } from "./types";

  let { env }: { env: string } = $props();

  let open = $state(true);
  let scripts: ScriptEntry[] = $state([]);
  let running = $state<string | null>(null);
  let output: string[] = $state([]);
  let outputFor = $state("");
  let error = $state("");

  // Editor modal state; null = closed. `original` tracks a rename.
  let editing: { original: string | null; name: string; content: string; onStart: boolean } | null = $state(null);

  async function refresh() {
    try {
      scripts = await invoke<ScriptEntry[]>("list_scripts", { name: env });
    } catch (e) {
      error = String(e);
    }
  }

  onMount(refresh);

  function newScript() {
    editing = { original: null, name: "", content: "#!/bin/sh\n", onStart: false };
  }

  function editScript(s: ScriptEntry) {
    editing = { original: s.name, name: s.name, content: s.content, onStart: s.onStart };
  }

  async function saveEditing() {
    if (!editing) return;
    error = "";
    try {
      if (editing.original && editing.original !== editing.name) {
        await invoke("delete_script", { name: env, scriptName: editing.original });
      }
      await invoke("save_script", {
        name: env,
        script: { name: editing.name.trim(), content: editing.content, onStart: editing.onStart },
      });
      editing = null;
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  async function removeScript(s: ScriptEntry) {
    if (!confirm(`Delete script '${s.name}'?`)) return;
    await invoke("delete_script", { name: env, scriptName: s.name }).catch((e) => (error = String(e)));
    await refresh();
  }

  async function toggleOnStart(s: ScriptEntry) {
    await invoke("save_script", {
      name: env,
      script: { ...s, onStart: !s.onStart },
    }).catch((e) => (error = String(e)));
    await refresh();
  }

  async function run(s: ScriptEntry) {
    running = s.name;
    outputFor = s.name;
    output = [];
    error = "";
    const chan = new Channel<string>();
    chan.onmessage = (line) => { output = [...output.slice(-400), line]; };
    try {
      const code = await invoke<number>("run_script", { name: env, scriptName: s.name, onOutput: chan });
      output = [...output, `[exit ${code}]`];
    } catch (e) {
      error = String(e);
    } finally {
      running = null;
    }
  }
</script>

<div class="scripts">
  <div class="head">
    <button class="fold" onclick={() => (open = !open)}>{open ? "▾" : "▸"} Scripts</button>
    <button class="add" title="New script" onclick={newScript}>+</button>
  </div>
  {#if open}
    {#each scripts as s (s.name)}
      <div class="row">
        <button
          class="run"
          disabled={running !== null}
          title="Run now"
          onclick={() => run(s)}
        >{running === s.name ? "⏳" : "▶"}</button>
        <span class="name" title={s.content}>{s.name}</span>
        <label class="onstart" title="Run automatically on every start">
          <input type="checkbox" checked={s.onStart} onchange={() => toggleOnStart(s)} />
          start
        </label>
        <button class="mini" title="Edit" onclick={() => editScript(s)}>✎</button>
        <button class="mini del" title="Delete" onclick={() => removeScript(s)}>✕</button>
      </div>
    {:else}
      <p class="empty">No scripts. + to add one.</p>
    {/each}
    {#if error}<p class="error">{error}</p>{/if}
    {#if output.length}
      <div class="out-head">
        <span>{outputFor}</span>
        <button class="mini" onclick={() => (output = [])}>✕</button>
      </div>
      <pre class="out">{output.join("")}</pre>
    {/if}
  {/if}
</div>

{#if editing}
  <div class="overlay">
    <div class="modal">
      <h3>{editing.original ? "Edit script" : "New script"}</h3>
      <label>Name <input bind:value={editing.name} placeholder="setup" /></label>
      <label>
        Script
        <textarea bind:value={editing.content} rows="12" spellcheck="false"></textarea>
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={editing.onStart} />
        Run on every environment start
      </label>
      {#if error}<p class="error">{error}</p>{/if}
      <div class="actions">
        <button onclick={() => (editing = null)}>Cancel</button>
        <button onclick={saveEditing} disabled={!editing.name.trim()}>Save</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .scripts { border-top: 1px solid #333; padding: 0.25rem 0; }
  .head { display: flex; align-items: center; }
  .fold { flex: 1; text-align: left; border: none; background: transparent; padding: 0.35em 0.6em; color: #ccc; }
  .add { border: none; background: transparent; padding: 0.35em 0.6em; }
  .add:hover, .fold:hover { color: #646cff; }
  .row { display: flex; align-items: center; gap: 0.25rem; padding: 0.1rem 0.5rem; }
  .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.85rem; }
  .run, .mini { border: none; background: transparent; padding: 0.15em 0.3em; }
  .run:hover:enabled { color: #4ade80; }
  .mini:hover { color: #646cff; }
  .del:hover { color: #f87171; }
  .onstart { display: flex; align-items: center; gap: 0.2rem; font-size: 0.7rem; color: #888; }
  .empty { color: #888; font-size: 0.8rem; padding: 0 0.6rem; }
  .error { color: #f87171; font-size: 0.8rem; padding: 0 0.6rem; }
  .out-head { display: flex; justify-content: space-between; align-items: center; padding: 0 0.5rem; color: #888; font-size: 0.75rem; }
  .out { background: #111; margin: 0.25rem 0.5rem; padding: 0.4rem; border-radius: 4px; max-height: 12rem; overflow: auto; font-size: 0.7rem; white-space: pre-wrap; }
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: grid; place-items: center; z-index: 10; }
  .modal { background: #1e1e1e; border-radius: 8px; padding: 1.25rem; width: 34rem; max-height: 85vh; overflow-y: auto; display: grid; gap: 0.75rem; }
  .modal label { display: grid; gap: 0.25rem; }
  .modal textarea { font-family: 'Cascadia Mono', Consolas, monospace; font-size: 0.8rem; background: #111; color: #ddd; border: 1px solid #333; border-radius: 4px; padding: 0.5rem; resize: vertical; }
  .modal .check { display: flex; align-items: center; gap: 0.5rem; }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
</style>
