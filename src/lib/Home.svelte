<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { EnvView } from "./types";
  import { runtimeLabel } from "./catalog";
  import Wizard from "./Wizard.svelte";

  let { onopen }: { onopen: (name: string) => void } = $props();
  let envs: EnvView[] = $state([]);
  let showWizard = $state(false);
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
    const un = listen("envs-changed", refresh);
    return () => un.then((f) => f());
  });
</script>

<main>
  <header>
    <h1>miniVE</h1>
    <button onclick={() => (showWizard = true)}>+ New Environment</button>
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

<style>
  header { display: flex; justify-content: space-between; align-items: center; padding: 1rem; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr)); gap: 1rem; padding: 1rem; }
  .card { border: 1px solid #333; border-radius: 8px; padding: 1rem; }
  .card[data-status="running"] .status { color: #4ade80; }
  .card[data-status="broken"] { opacity: 0.6; }
  .actions { display: flex; gap: 0.5rem; margin-top: 0.5rem; }
  .error { color: #f87171; padding: 0 1rem; }
</style>
