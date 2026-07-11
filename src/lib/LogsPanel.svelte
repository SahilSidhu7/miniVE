<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke, Channel } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type { EnvView } from "./types";

  let { onclose }: { onclose: () => void } = $props();

  type Mode = "app" | "container";
  let mode: Mode = $state("app");
  let appLines: string[] = $state([]);
  let containerLines: string[] = $state([]);
  let envs: EnvView[] = $state([]);
  let selectedEnv = $state("");
  let unlisten: UnlistenFn | null = null;
  let streamGen = 0;

  async function loadAppLogs() {
    appLines = await invoke<string[]>("get_backend_logs");
    unlisten = await listen<string>("backend-log", (e) => {
      appLines = [...appLines.slice(-500), e.payload];
    });
  }

  async function loadEnvs() {
    envs = await invoke<EnvView[]>("list_envs");
    if (envs.length && !selectedEnv) selectedEnv = envs[0].name;
  }

  async function streamContainerLogs() {
    if (!selectedEnv) return;
    containerLines = [];
    streamGen += 1;
    const gen = streamGen;
    const out = new Channel<string>();
    out.onmessage = (line) => {
      if (gen !== streamGen) return;
      containerLines = [...containerLines.slice(-500), line];
    };
    invoke("stream_container_logs", { name: selectedEnv, onOutput: out }).catch((e) => {
      if (gen !== streamGen) return;
      containerLines = [...containerLines, `[error] ${String(e)}`];
    });
  }

  async function switchMode(next: Mode) {
    mode = next;
    if (next === "container" && envs.length === 0) await loadEnvs();
    if (next === "container") await streamContainerLogs();
  }

  onMount(loadAppLogs);
  onDestroy(() => { if (unlisten) unlisten(); });
</script>

<div class="overlay">
  <div class="modal">
    <h2>Logs</h2>
    <div class="tabs">
      <button class:active={mode === "app"} onclick={() => switchMode("app")}>App</button>
      <button class:active={mode === "container"} onclick={() => switchMode("container")}>Container</button>
    </div>
    {#if mode === "container"}
      <label>Environment
        <select bind:value={selectedEnv} onchange={streamContainerLogs}>
          {#each envs as e}<option value={e.name}>{e.name}</option>{/each}
        </select>
      </label>
    {/if}
    <pre class="log">{(mode === "app" ? appLines : containerLines).join("\n")}</pre>
    <div class="actions">
      <button onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: grid; place-items: center; }
  .modal { background: #1e1e1e; border-radius: 8px; padding: 1.5rem; width: 42rem; max-height: 85vh; overflow-y: auto; display: grid; gap: 0.75rem; }
  .tabs { display: flex; gap: 0.5rem; }
  .tabs button.active { font-weight: bold; text-decoration: underline; }
  .log { background: #111; padding: 0.5rem; border-radius: 4px; max-height: 24rem; overflow-y: auto; font-size: 0.75rem; white-space: pre-wrap; }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
</style>
