<script lang="ts">
  import { invoke, Channel } from "@tauri-apps/api/core";
  import type { PortMap, PackagePreset, FamilyVersions, LangSpec } from "./types";
  import { loadCatalog, loadPinned, distroEntries, languageFamilies } from "./catalog";
  import { onMount } from "svelte";

  let { onclose, oncreated }: { onclose: () => void; oncreated: (name: string) => void } = $props();

  let entries: { label: string; image: string }[] = $state([]);
  let langFamilies: FamilyVersions[] = $state([]);
  let name = $state("");
  let image = $state("");
  let preset: PackagePreset = $state("minimal");
  let ports: PortMap[] = $state([]);
  let gitUrl = $state("");
  let dockerAccess = $state(false);
  let busy = $state(false);
  let log: string[] = $state([]);
  let error = $state("");

  // Language picker state: key → { checked, version }.
  let langSel: Record<string, { checked: boolean; version: string }> = $state({});

  // How the version dropdown behaves per language. "major": dedupe to major
  // versions (that's what the installer honors); "exact": full version list;
  // "none": the distro's package manager decides — no dropdown.
  const versionMode: Record<string, "major" | "exact" | "none"> = {
    node: "major",
    openjdk: "major",
    golang: "exact",
    rust: "exact",
    python: "none",
    ruby: "none",
    php: "none",
  };

  function versionsFor(f: FamilyVersions): string[] {
    if (versionMode[f.key] === "major") {
      const majors: string[] = [];
      for (const v of f.versions) {
        const m = v.split(".")[0];
        if (!majors.includes(m)) majors.push(m);
      }
      return majors;
    }
    return f.versions;
  }

  onMount(async () => {
    await Promise.all([loadCatalog(), loadPinned()]);
    entries = distroEntries();
    langFamilies = languageFamilies();
    if (entries.length) image = entries[0].image;
    for (const f of langFamilies) {
      langSel[f.key] = { checked: false, version: versionsFor(f)[0] ?? "" };
    }
  });

  function addPort() { ports = [...ports, { host: 8000, container: 8000 }]; }
  function removePort(i: number) { ports = ports.filter((_, idx) => idx !== i); }

  function selectedLanguages(): LangSpec[] {
    return Object.entries(langSel)
      .filter(([, s]) => s.checked)
      .map(([key, s]) => ({ key, version: s.version }));
  }

  async function create() {
    busy = true;
    error = "";
    log = [];
    const progress = new Channel<string>();
    progress.onmessage = (line) => { log = [...log.slice(-200), line]; };
    try {
      await invoke("create_env", {
        spec: { name, image, ports, preset, languages: selectedLanguages(), dockerAccess },
        onProgress: progress,
      });
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
    <label>Base system
      <select bind:value={image} disabled={busy}>
        {#each entries as e}<option value={e.image}>{e.label}</option>{/each}
      </select>
    </label>
    <fieldset disabled={busy}>
      <legend>Languages (installed on top of the base)</legend>
      {#each langFamilies as f (f.key)}
        <div class="lang-row">
          <label class="lang-check">
            <input type="checkbox" bind:checked={langSel[f.key].checked} />
            {f.displayName}
          </label>
          {#if langSel[f.key].checked}
            {#if versionMode[f.key] !== "none"}
              <select bind:value={langSel[f.key].version}>
                {#each versionsFor(f) as v}<option value={v}>{v}</option>{/each}
              </select>
            {:else}
              <span class="hint">distro version</span>
            {/if}
          {/if}
        </div>
      {/each}
      <p class="hint">Installed by a startup script that re-runs (cheaply) on every start.</p>
    </fieldset>
    <label>Packages
      <select bind:value={preset} disabled={busy}>
        <option value="none">None</option>
        <option value="minimal">Minimal (git, curl)</option>
        <option value="full">Full (+ vim, unzip, build tools)</option>
        <option value="essentials">Dev essentials (update + upgrade + common dev tools)</option>
      </select>
    </label>
    <label class="lang-check">
      <input type="checkbox" bind:checked={dockerAccess} disabled={busy} />
      Docker CLI (shares the host's Docker — containers started inside run on the host)
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
  .lang-row { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.25rem; }
  .lang-row select { min-width: 7rem; }
  .lang-check { display: flex; flex-direction: row; align-items: center; gap: 0.5rem; }
  .hint { color: #888; font-size: 0.8rem; margin: 0.25rem 0 0; }
  .port-row { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.25rem; }
  .log { background: #111; padding: 0.5rem; border-radius: 4px; max-height: 10rem; overflow-y: auto; font-size: 0.75rem; }
  .error { color: #f87171; }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
</style>
