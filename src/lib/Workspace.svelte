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
  let sidebarOpen = $state(true);
  let split = $state(false);

  function closeTab(t: number) {
    tabs = tabs.filter((x) => x !== t);
    if (active === t) active = tabs[0];
  }

  onMount(async () => {
    const envs = await invoke<EnvView[]>("list_envs");
    env = envs.find((e) => e.name === name);
  });
</script>

<div class="workspace">
  <header>
    <button
      class="icon"
      aria-label={sidebarOpen ? "Hide file tree" : "Show file tree"}
      title={sidebarOpen ? "Hide file tree" : "Show file tree"}
      onclick={() => (sidebarOpen = !sidebarOpen)}
    >{sidebarOpen ? "◧" : "◨"}</button>
    <button onclick={onclose}>← Environments</button>
    <h2>{name}</h2>
    <span class="spacer"></span>
    <button class:toggled={split} onclick={() => (split = !split)}>
      {split ? "Tabs" : "Split view"}
    </button>
  </header>
  <div class="body">
    <aside class:collapsed={!sidebarOpen}><FileTree env={name} /></aside>
    <section class="main">
      <nav class="tabs">
        {#each tabs as t (t)}
          <div class="tab" class:active={t === active}>
            <button class="tab-label" onclick={() => (active = t)}>Terminal {t}</button>
            {#if tabs.length > 1}
              <button
                class="tab-close"
                aria-label={`Close terminal ${t}`}
                onclick={() => closeTab(t)}
              >✕</button>
            {/if}
          </div>
        {/each}
        <button class="tab-add" aria-label="New terminal" onclick={() => { tabs = [...tabs, nextTab]; active = nextTab; nextTab++; }}>+</button>
      </nav>
      <div class="terms" class:split>
        {#each tabs as t (t)}
          <!-- focusin: clicking into a terminal focuses xterm's textarea, which
               bubbles here — keeps `active` in sync with the pane being typed in -->
          <div
            class="term-holder"
            class:focused={split && t === active}
            style:display={split || t === active ? "block" : "none"}
            onfocusin={() => (active = t)}
          >
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
  header { display: flex; gap: 0.75rem; align-items: center; padding: 0.5rem 1rem; border-bottom: 1px solid #333; }
  header h2 { margin: 0; font-size: 1.1rem; }
  .spacer { flex: 1; }
  .icon { padding: 0.5em 0.7em; }
  .toggled { border-color: #646cff; background: #2a2a4a; }
  .body { display: flex; flex: 1; min-height: 0; }
  aside { width: 16rem; border-right: 1px solid #333; overflow-y: auto; }
  /* display:none (not {#if}) so the tree keeps its state while hidden */
  aside.collapsed { display: none; }
  .main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .tabs { display: flex; gap: 2px; padding: 0.25rem; }
  .tab { display: flex; }
  .tab.active { background: #333; border-radius: 6px; }
  .tab-label, .tab-close, .tab-add { border: none; background: transparent; padding: 0.35em 0.7em; }
  .tab-close:hover, .tab-add:hover { color: #f87171; }
  .tab-add:hover { color: #646cff; }
  .terms { flex: 1; min-height: 0; }
  .terms.split {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr));
    grid-auto-rows: 1fr;
    gap: 2px;
    background: #333;
  }
  .terms.split .term-holder { min-height: 0; border: 1px solid transparent; }
  .terms.split .term-holder.focused { border-color: #646cff; }
  .term-holder { height: 100%; }
</style>
