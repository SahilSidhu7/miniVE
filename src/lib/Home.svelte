<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { EnvView } from "./types";
  import { runtimeLabel, loadCatalog } from "./catalog";
  import Wizard from "./Wizard.svelte";
  import ManageImages from "./ManageImages.svelte";
  import LogsPanel from "./LogsPanel.svelte";

  let { onopen }: { onopen: (name: string) => void } = $props();
  let envs: EnvView[] = $state([]);
  let showWizard = $state(false);
  let showImages = $state(false);
  let showLogs = $state(false);
  let error = $state("");

  async function refresh() {
    try {
      envs = await invoke<EnvView[]>("list_envs");
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function act(cmd: string, name: string) {
    try {
      await invoke(cmd, { name });
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    refresh();
    // catalog.ts's `families` is plain module state, not a Svelte store — once
    // it loads, force a re-render so cards already showing raw "python:3.12"
    // pick up the resolved "Python 3.12" label.
    loadCatalog().then(() => { envs = [...envs]; });
    const un = listen("envs-changed", refresh);
    return () => un.then((f) => f());
  });
</script>

<main>
  <header>
    <h1>miniVE</h1>
    <div class="header-actions">
      <button onclick={() => (showImages = true)}>Manage Images</button>
      <button class="primary" onclick={() => (showWizard = true)}>+ New Environment</button>
      <button onclick={() => (showLogs = true)}>Logs</button>
    </div>
  </header>

  {#if error}<p class="error">{error}</p>{/if}

  <div class="cards">
    {#each envs as env (env.name)}
      <div class="card" data-status={env.status}>
        <h2>{env.name}</h2>
        <p>{runtimeLabel(env.image)}</p>
        <p class="status">{env.status}</p>
        {#if env.ports.length}
          <p class="ports">{env.ports.map((p) => `localhost:${p.host}→${p.container}`).join(", ")}</p>
        {/if}
        <div class="actions">
          {#if env.status === "running"}
            <button onclick={() => onopen(env.name)}>Open</button>
            <button onclick={() => act("stop_env", env.name)}>Stop</button>
          {:else if env.status === "stopped"}
            <button onclick={() => act("start_env", env.name)}>Start</button>
          {/if}
          <button
            onclick={() => {
              if (confirm(`Delete '${env.name}' and all its files?`)) act("delete_env", env.name);
            }}>Delete</button>
        </div>
      </div>
    {:else}
      <p>No environments yet. Create one.</p>
    {/each}
  </div>
</main>

{#if showWizard}
  <Wizard onclose={() => (showWizard = false)} oncreated={(name) => { showWizard = false; refresh(); onopen(name); }} />
{/if}

{#if showImages}
  <ManageImages onclose={() => (showImages = false)} />
{/if}

{#if showLogs}
  <LogsPanel onclose={() => (showLogs = false)} />
{/if}

<style>
  header { display: flex; justify-content: space-between; align-items: center; padding: 1rem; }
  .header-actions { display: flex; gap: 0.5rem; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr)); gap: 1rem; padding: 1rem; }
  .card { border: 1px solid #333; border-radius: 8px; padding: 1rem; transition: border-color 0.15s; }
  .card:hover { border-color: #555; }
  .status::before { content: "●"; margin-right: 0.35rem; font-size: 0.7em; vertical-align: 1px; }
  .card .status { color: #9ca3af; }
  .card[data-status="running"] .status { color: #4ade80; }
  .card[data-status="broken"] { opacity: 0.6; }
  .actions { display: flex; gap: 0.5rem; margin-top: 0.5rem; }
  .error { color: #f87171; padding: 0 1rem; }
</style>
