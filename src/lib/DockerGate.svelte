<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  let { onretry }: { onretry: () => void } = $props();
</script>

<div class="gate">
  <h1>miniVE needs Docker</h1>
  <p>Environments run as Docker containers. Install and start Docker Desktop, then retry.</p>
  <button onclick={() => openUrl("https://www.docker.com/products/docker-desktop/")}>Get Docker Desktop</button>
  <button onclick={onretry}>Re-check</button>
</div>

<style>
  .gate { max-width: 28rem; margin: 20vh auto; text-align: center; display: grid; gap: 0.75rem; }
</style>
