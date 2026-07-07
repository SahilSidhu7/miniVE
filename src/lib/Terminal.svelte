<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { invoke, Channel } from "@tauri-apps/api/core";

  let { env }: { env: string } = $props();
  let el: HTMLDivElement;
  let sessionId: number | null = null;
  let ro: ResizeObserver;

  onMount(async () => {
    const term = new Terminal({ fontSize: 13, cursorBlink: true, theme: { background: "#111" } });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);
    fit.fit();

    const chan = new Channel<string>();
    chan.onmessage = (d) => term.write(d);
    try {
      sessionId = await invoke<number>("open_terminal", { name: env, onData: chan });
    } catch (e) {
      term.write(`\r\n\x1b[31m${String(e)}\x1b[0m\r\n`);
      return;
    }
    term.onData((d) => invoke("write_terminal", { id: sessionId, data: d }));
    term.onResize(({ cols, rows }) => invoke("resize_terminal", { id: sessionId, cols, rows }));
    invoke("resize_terminal", { id: sessionId, cols: term.cols, rows: term.rows });
    ro = new ResizeObserver(() => fit.fit());
    ro.observe(el);
  });

  onDestroy(() => {
    ro?.disconnect();
    if (sessionId !== null) invoke("close_terminal", { id: sessionId });
  });
</script>

<div bind:this={el} class="term"></div>

<style>
  .term { height: 100%; width: 100%; background: #111; }
</style>
