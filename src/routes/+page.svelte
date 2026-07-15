<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import DockerGate from "$lib/DockerGate.svelte";
  import Home from "$lib/Home.svelte";
  import Workspace from "$lib/Workspace.svelte";
  import Popout from "$lib/Popout.svelte";
  import { popins, type Popin } from "$lib/popin";

  // Popped-out terminal windows load the same SPA with ?term=<session>&env=<name>.
  const q = new URLSearchParams(window.location.search);
  const popTerm = q.get("term");
  const popEnv = q.get("env");
  const isPopout = popTerm !== null && popEnv !== null;

  let dockerOk: boolean | null = $state(null);
  let dockerLost = $state(false);
  let openEnv: string | null = $state(null);

  async function check() {
    dockerOk = await invoke<boolean>("docker_status");
  }

  onMount(() => {
    if (isPopout) return;
    check();
    const un1 = listen("docker-lost", () => (dockerLost = true));
    const un2 = listen("docker-back", () => (dockerLost = false));
    // A popped-out terminal asking to come home: queue it for the env's
    // Workspace to adopt, opening that workspace if it isn't the current one.
    const un3 = listen<Popin>("term-popin", (e) => {
      popins.update((q) => [...q, e.payload]);
      if (openEnv !== e.payload.env) openEnv = e.payload.env;
    });
    return () => { un1.then((f) => f()); un2.then((f) => f()); un3.then((f) => f()); };
  });
</script>

{#if isPopout}
  <Popout env={popEnv!} session={Number(popTerm)} />
{:else}
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
{/if}
