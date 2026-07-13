<script lang="ts">
  import Terminal from "./Terminal.svelte";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  let { env, session }: { env: string; session: number } = $props();

  onMount(() => {
    // Closing the window kills the session, same as closing the tab would.
    const un = getCurrentWindow().onCloseRequested(async () => {
      await invoke("close_terminal", { id: session }).catch(() => {});
    });
    return () => {
      un.then((f) => f());
    };
  });
</script>

<div class="pop">
  <Terminal {env} attach={session} />
</div>

<style>
  .pop { height: 100vh; }
</style>
