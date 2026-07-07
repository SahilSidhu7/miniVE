<script lang="ts">
  import Terminal from "./Terminal.svelte";
  import FileTree from "./FileTree.svelte";
  import Preview from "./Preview.svelte";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { EnvView } from "./types";

  let { name, onclose }: { name: string; onclose: () => void } = $props();
  let tabs: number[] = $state([1]);
  let active = $state(1);
  let nextTab = 2;
  let env: EnvView | undefined = $state();

  onMount(async () => {
    const envs = await invoke<EnvView[]>("list_envs");
    env = envs.find((e) => e.name === name);
  });
</script>

<div class="workspace">
  <header>
    <button onclick={onclose}>← Environments</button>
    <h2>{name}</h2>
  </header>
  <div class="body">
    <aside><FileTree env={name} /></aside>
    <section class="main">
      <nav class="tabs">
        {#each tabs as t (t)}
          <div class="tab" class:active={t === active}>
            <button class="tab-label" onclick={() => (active = t)}>Terminal {t}</button>
            {#if tabs.length > 1}
              <button
                class="tab-close"
                aria-label={`Close terminal ${t}`}
                onclick={() => { tabs = tabs.filter((x) => x !== t); if (active === t) active = tabs[0]; }}
              >✕</button>
            {/if}
          </div>
        {/each}
        <button onclick={() => { tabs = [...tabs, nextTab]; active = nextTab; nextTab++; }}>+</button>
      </nav>
      <div class="terms">
        {#each tabs as t (t)}
          <div class="term-holder" style:display={t === active ? "block" : "none"}>
            <Terminal env={name} />
          </div>
        {/each}
      </div>
      {#if env && env.ports.length}
        <Preview ports={env.ports} />
      {/if}
    </section>
  </div>
</div>

<style>
  .workspace { display: flex; flex-direction: column; height: 100vh; }
  header { display: flex; gap: 1rem; align-items: center; padding: 0.5rem 1rem; border-bottom: 1px solid #333; }
  .body { display: flex; flex: 1; min-height: 0; }
  aside { width: 16rem; border-right: 1px solid #333; overflow-y: auto; }
  .main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .tabs { display: flex; gap: 2px; padding: 0.25rem; }
  .tab { display: flex; }
  .tab.active { background: #333; }
  .terms { flex: 1; min-height: 0; }
  .term-holder { height: 100%; }
</style>
