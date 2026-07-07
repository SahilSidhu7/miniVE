<script lang="ts">
  import { invoke, Channel } from "@tauri-apps/api/core";
  import { RUNTIMES, type PortMap } from "./types";

  let { onclose, oncreated }: { onclose: () => void; oncreated: (name: string) => void } = $props();

  let name = $state("");
  let image = $state(RUNTIMES[0].image);
  let ports: PortMap[] = $state([]);
  let gitUrl = $state("");
  let busy = $state(false);
  let log: string[] = $state([]);
  let error = $state("");

  function addPort() { ports = [...ports, { host: 8000, container: 8000 }]; }
  function removePort(i: number) { ports = ports.filter((_, idx) => idx !== i); }

  async function create() {
    busy = true;
    error = "";
    log = [];
    const progress = new Channel<string>();
    progress.onmessage = (line) => { log = [...log.slice(-200), line]; };
    try {
      await invoke("create_env", { spec: { name, image, ports }, onProgress: progress });
      if (gitUrl.trim()) {
        const out = new Channel<string>();
        out.onmessage = (line) => { log = [...log.slice(-200), line]; };
        const code = await invoke<number>("clone_repo", { name, url: gitUrl.trim(), onOutput: out });
        if (code !== 0) { error = `git clone exited with code ${code}`; busy = false; return; }
      }
      oncreated(name);
    } catch (e) {
      error = String(e);
      busy = false;
    }
  }
</script>

<div class="overlay">
  <div class="modal">
    <h2>New Environment</h2>
    <label>Name <input bind:value={name} placeholder="my-project" disabled={busy} /></label>
    <label>Runtime
      <select bind:value={image} disabled={busy}>
        {#each RUNTIMES as r}<option value={r.image}>{r.label}</option>{/each}
      </select>
    </label>
    <fieldset disabled={busy}>
      <legend>Ports (host → container)</legend>
      {#each ports as p, i}
        <div class="port-row">
          <input type="number" bind:value={p.host} min="1" max="65535" />
          →
          <input type="number" bind:value={p.container} min="1" max="65535" />
          <button onclick={() => removePort(i)}>✕</button>
        </div>
      {/each}
      <button onclick={addPort}>+ Add port</button>
    </fieldset>
    <label>Git URL (optional) <input bind:value={gitUrl} placeholder="https://github.com/user/repo.git" disabled={busy} /></label>

    {#if log.length}
      <pre class="log">{log.join("\n")}</pre>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}

    <div class="actions">
      <button onclick={onclose} disabled={busy}>Cancel</button>
      <button onclick={create} disabled={busy || !name}>{busy ? "Creating…" : "Create"}</button>
    </div>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: grid; place-items: center; }
  .modal { background: #1e1e1e; border-radius: 8px; padding: 1.5rem; width: 32rem; max-height: 85vh; overflow-y: auto; display: grid; gap: 0.75rem; }
  label { display: grid; gap: 0.25rem; }
  .port-row { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.25rem; }
  .log { background: #111; padding: 0.5rem; border-radius: 4px; max-height: 10rem; overflow-y: auto; font-size: 0.75rem; }
  .error { color: #f87171; }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
</style>
