<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import DockerGate from "$lib/DockerGate.svelte";
  import Home from "$lib/Home.svelte";
  import Workspace from "$lib/Workspace.svelte";

  let dockerOk: boolean | null = $state(null);
  let dockerLost = $state(false);
  let openEnv: string | null = $state(null);

  async function check() {
    dockerOk = await invoke<boolean>("docker_status");
  }

  onMount(() => {
    check();
    const un1 = listen("docker-lost", () => (dockerLost = true));
    const un2 = listen("docker-back", () => (dockerLost = false));
    return () => { un1.then((f) => f()); un2.then((f) => f()); };
  });
</script>

{#if dockerLost}
  <div class="banner">Docker is not running — start Docker Desktop. Reconnecting…</div>
{/if}

{#if dockerOk === null}
  <p class="center">Checking Docker…</p>
{:else if !dockerOk}
  <DockerGate onretry={check} />
{:else if openEnv}
  <Workspace name={openEnv} onclose={() => (openEnv = null)} />
{:else}
  <Home onopen={(n) => (openEnv = n)} />
{/if}
