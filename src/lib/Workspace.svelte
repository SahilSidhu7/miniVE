<script lang="ts">
  import Terminal from "./Terminal.svelte";
  import FileTree from "./FileTree.svelte";
  import Preview from "./Preview.svelte";
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import type { EnvView } from "./types";

  type Tab = { key: number; session: number | null; popped: boolean; w: number };

  let { name, onclose }: { name: string; onclose: () => void } = $props();
  let tabs: Tab[] = $state([{ key: 1, session: null, popped: false, w: 1 }]);
  let active = $state(1);
  let nextTab = 2;
  let env: EnvView | undefined = $state();
  let sidebarOpen = $state(true);
  let sidebarW = $state(256);
  let split = $state(false);
  let termsCollapsed = $state(false);
  let termsEl: HTMLDivElement | undefined = $state();

  function addTab() {
    tabs = [...tabs, { key: nextTab, session: null, popped: false, w: 1 }];
    active = nextTab;
    nextTab++;
  }

  function closeTab(t: Tab) {
    tabs = tabs.filter((x) => x.key !== t.key);
    if (active === t.key) active = tabs[0]?.key ?? 0;
  }

  async function popOut(t: Tab) {
    if (t.session === null) return;
    // Mark first so the Terminal's onDestroy detaches instead of killing
    // the session, then remove the tab — the new window re-attaches by id.
    t.popped = true;
    await tick();
    new WebviewWindow(`term-${t.session}`, {
      url: `/?term=${t.session}&env=${encodeURIComponent(name)}`,
      title: `${name} — Terminal ${t.key}`,
      width: 900,
      height: 560,
    });
    closeTab(t);
  }

  function trackDrag(e: PointerEvent, onmove: (ev: PointerEvent) => void) {
    e.preventDefault();
    const up = () => {
      window.removeEventListener("pointermove", onmove);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", onmove);
    window.addEventListener("pointerup", up);
  }

  function dragSidebar(e: PointerEvent) {
    const startX = e.clientX;
    const startW = sidebarW;
    trackDrag(e, (ev) => {
      sidebarW = Math.min(600, Math.max(140, startW + ev.clientX - startX));
    });
  }

  function dragDivider(e: PointerEvent, i: number) {
    if (!termsEl) return;
    const total = tabs.reduce((s, t) => s + t.w, 0);
    const pxPerW = termsEl.clientWidth / total;
    const startX = e.clientX;
    const l = tabs[i];
    const r = tabs[i + 1];
    const lw = l.w;
    const rw = r.w;
    trackDrag(e, (ev) => {
      const want = (ev.clientX - startX) / pxPerW;
      const d = Math.max(-(lw - 0.15), Math.min(rw - 0.15, want));
      l.w = lw + d;
      r.w = rw - d;
    });
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
    <aside class:collapsed={!sidebarOpen} style:width={sidebarOpen ? `${sidebarW}px` : undefined}>
      <FileTree env={name} />
    </aside>
    {#if sidebarOpen}
      <div
        class="vdrag"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize file tree"
        onpointerdown={dragSidebar}
      ></div>
    {/if}
    <section class="main">
      <nav class="tabs">
        <button
          class="tab-collapse"
          aria-label={termsCollapsed ? "Expand terminals" : "Collapse terminals"}
          title={termsCollapsed ? "Expand terminals" : "Collapse terminals"}
          onclick={() => (termsCollapsed = !termsCollapsed)}
        >{termsCollapsed ? "▸" : "▾"}</button>
        {#each tabs as t (t.key)}
          <div class="tab" class:active={t.key === active}>
            <button class="tab-label" onclick={() => { active = t.key; termsCollapsed = false; }}>Terminal {t.key}</button>
            <button
              class="tab-pop"
              disabled={t.session === null}
              aria-label={`Pop out terminal ${t.key}`}
              title="Pop out into its own window"
              onclick={() => popOut(t)}
            >⧉</button>
            {#if tabs.length > 1}
              <button
                class="tab-close"
                aria-label={`Close terminal ${t.key}`}
                onclick={() => closeTab(t)}
              >✕</button>
            {/if}
          </div>
        {/each}
        <button class="tab-add" aria-label="New terminal" onclick={addTab}>+</button>
      </nav>
      <div class="terms" class:split class:collapsed={termsCollapsed} bind:this={termsEl}>
        {#each tabs as t, i (t.key)}
          <!-- focusin: clicking into a terminal focuses xterm's textarea, which
               bubbles here — keeps `active` in sync with the pane being typed in -->
          <div
            class="term-holder"
            class:focused={split && t.key === active}
            style:display={split || t.key === active ? "block" : "none"}
            style:flex={split ? `${t.w} 1 0%` : undefined}
            onfocusin={() => (active = t.key)}
          >
            <Terminal env={name} popped={t.popped} onready={(id) => (t.session = id)} />
          </div>
          {#if split && i < tabs.length - 1}
            <div
              class="divider"
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize terminals"
              onpointerdown={(e) => dragDivider(e, i)}
            ></div>
          {/if}
        {/each}
        {#if tabs.length === 0}
          <p class="empty">No terminals — press + to open one.</p>
        {/if}
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
  aside { border-right: 1px solid #333; overflow-y: auto; flex-shrink: 0; }
  /* display:none (not {#if}) so the tree keeps its state while hidden */
  aside.collapsed { display: none; }
  .vdrag { flex: 0 0 5px; cursor: col-resize; margin-left: -3px; }
  .vdrag:hover { background: #646cff44; }
  .main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .tabs { display: flex; gap: 2px; padding: 0.25rem; align-items: center; }
  .tab { display: flex; }
  .tab.active { background: #333; border-radius: 6px; }
  .tab-label, .tab-close, .tab-add, .tab-pop, .tab-collapse { border: none; background: transparent; padding: 0.35em 0.7em; }
  .tab-pop { padding: 0.35em 0.4em; }
  .tab-pop:disabled { opacity: 0.3; }
  .tab-pop:hover:enabled { color: #646cff; }
  .tab-close:hover { color: #f87171; }
  .tab-add:hover, .tab-collapse:hover { color: #646cff; }
  /* display:none keeps xterm instances (and their sessions) alive while hidden */
  .terms { flex: 1; min-height: 0; }
  .terms.collapsed { display: none; }
  .terms.split {
    display: flex;
    background: #333;
  }
  .terms.split .term-holder { min-width: 8rem; min-height: 0; border: 1px solid transparent; }
  .terms.split .term-holder.focused { border-color: #646cff; }
  .divider { flex: 0 0 4px; cursor: col-resize; background: #333; }
  .divider:hover { background: #646cff; }
  .term-holder { height: 100%; }
  .empty { padding: 1rem; color: #888; }
</style>
