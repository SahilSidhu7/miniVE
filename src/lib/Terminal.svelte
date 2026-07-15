<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { invoke, Channel } from "@tauri-apps/api/core";

  let {
    env,
    attach = null,
    popped = false,
    onready,
  }: {
    env: string;
    /** Existing session id to attach to instead of opening a new shell. */
    attach?: number | null;
    /** True while this view is being handed off to a pop-out window:
        on destroy, detach this viewer instead of killing the session. */
    popped?: boolean;
    onready?: (id: number) => void;
  } = $props();
  let el: HTMLDivElement;
  let sessionId: number | null = null;
  let chan: Channel<string>;
  let ro: ResizeObserver;

  onMount(async () => {
    // Git-bash (mintty on Windows) look: console palette, block cursor,
    // Cascadia/Consolas type. Pairs with the green/yellow PS1 set backend-side.
    const term = new Terminal({
      fontSize: 14,
      fontFamily: "'Cascadia Mono', Consolas, 'Courier New', monospace",
      cursorBlink: true,
      cursorStyle: "block",
      theme: {
        background: "#0c0c0c",
        foreground: "#cccccc",
        cursor: "#cccccc",
        black: "#0c0c0c",
        red: "#c50f1f",
        green: "#13a10e",
        yellow: "#c19c00",
        blue: "#0037da",
        magenta: "#881798",
        cyan: "#3a96dd",
        white: "#cccccc",
        brightBlack: "#767676",
        brightRed: "#e74856",
        brightGreen: "#16c60c",
        brightYellow: "#f9f1a5",
        brightBlue: "#3b78ff",
        brightMagenta: "#b4009e",
        brightCyan: "#61d6d6",
        brightWhite: "#f2f2f2",
      },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);
    fit.fit();

    chan = new Channel<string>();
    chan.onmessage = (d) => term.write(d);
    try {
      if (attach !== null) {
        sessionId = attach;
        await invoke("attach_terminal", { id: attach, onData: chan });
      } else {
        sessionId = await invoke<number>("open_terminal", { name: env, onData: chan });
      }
    } catch (e) {
      term.write(`\r\n\x1b[31m${String(e)}\x1b[0m\r\n`);
      return;
    }
    onready?.(sessionId);
    term.onData((d) => invoke("write_terminal", { id: sessionId, data: d }).catch(() => {}));
    term.onResize(({ cols, rows }) => invoke("resize_terminal", { id: sessionId, cols, rows }).catch(() => {}));
    invoke("resize_terminal", { id: sessionId, cols: term.cols, rows: term.rows }).catch(() => {});
    ro = new ResizeObserver(() => fit.fit());
    ro.observe(el);
  });

  onDestroy(() => {
    ro?.disconnect();
    if (sessionId === null) return;
    if (popped) {
      invoke("detach_terminal", { id: sessionId, channelId: chan.id }).catch(() => {});
    } else {
      invoke("close_terminal", { id: sessionId });
    }
  });
</script>

<div bind:this={el} class="term"></div>

<style>
  .term { height: 100%; width: 100%; background: #0c0c0c; }
</style>
