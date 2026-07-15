<script lang="ts">
  import Terminal from "./Terminal.svelte";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { emitTo } from "@tauri-apps/api/event";

  let { env, session }: { env: string; session: number } = $props();

  // Set while handing the session back to the main window, so the
  // close-requested handler doesn't kill it on the way out.
  let poppingIn = false;

  async function popIn() {
    poppingIn = true;
    await emitTo("main", "term-popin", { env, session });
    await getCurrentWindow().destroy();
  }

  onMount(() => {
    // Closing the window kills the session, same as closing the tab would —
    // unless the session was just handed back via popIn().
    const un = getCurrentWindow().onCloseRequested(async () => {
      if (!poppingIn) await invoke("close_terminal", { id: session }).catch(() => {});
    });
    return () => {
      un.then((f) => f());
    };
  });
</script>

<div class="pop">
  <div class="bar">
    <span class="title">{env}</span>
    <button onclick={popIn} title="Move this terminal back into the main window">⇤ Pop back in</button>
    <button onclick={() => getCurrentWindow().close()} title="Close (ends the session)">✕</button>
  </div>
  <div class="term-wrap">
    <Terminal {env} attach={session} />
  </div>
</div>

<style>
  .pop { height: 100vh; display: flex; flex-direction: column; }
  .bar { display: flex; gap: 0.5rem; align-items: center; padding: 0.25rem 0.5rem; border-bottom: 1px solid #333; }
  .title { flex: 1; color: #888; font-size: 0.85rem; }
  .bar button { border: none; background: transparent; padding: 0.3em 0.6em; }
  .bar button:hover { color: #646cff; }
  .term-wrap { flex: 1; min-height: 0; }
</style>
