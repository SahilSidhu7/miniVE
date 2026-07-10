# Backend & Container Logs Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the dev an in-app panel to view backend app logs and per-container docker logs, replacing the app's current ad-hoc `eprintln!` calls with structured, retrievable logging.

**Architecture:** Add `tracing` + `tracing-subscriber` + `tracing-appender`, initialized inside Tauri's `.setup()` closure (where an `AppHandle` is available). A custom `tracing_subscriber::Layer` pushes formatted lines into a capped in-memory ring buffer (served on demand) and emits a `backend-log` Tauri event for live updates; a second layer writes to a daily-rotating log file for persistence across restarts. Container logs reuse bollard's `Docker::logs()` streamed through the same `Channel<String>` pattern already used for image-pull progress. Frontend gets one new "Logs" modal (same overlay pattern as the existing Wizard) with an App/Container toggle.

**Tech Stack:** Rust (`tracing`, `tracing-subscriber`, `tracing-appender` — new deps), bollard 0.18 (`logs()`, already a dependency), Svelte 5 (runes).

**Depends on:** none — independent of `docs/superpowers/plans/2026-07-10-runtime-catalog.md`, can be implemented and shipped on its own.

## Global Constraints

- Logging must never crash the app — file write failures are swallowed (spec: `docs/superpowers/specs/2026-07-10-runtime-catalog-and-logs-design.md`, "Error handling").
- Ring buffer capped at ~2000 lines (spec: "Backend app logs").
- Replace the existing `eprintln!` call sites (`src-tauri/src/registry.rs:60`, `:63`, `src-tauri/src/lib.rs:57`) as part of this work — no stray ad-hoc logging left after this plan.

---

### Task 1: Logging infrastructure — ring buffer + file layer

**Files:**
- Create: `src-tauri/src/logging.rs`
- Modify: `src-tauri/Cargo.toml` (add `tracing`, `tracing-subscriber`, `tracing-appender`)

**Interfaces:**
- Produces: `pub type LogBuffer = std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<String>>>`, `pub fn init(app: &tauri::AppHandle, log_dir: std::path::PathBuf) -> LogBuffer`, `pub fn push_and_format(buffer: &LogBuffer, max_lines: usize, line: String) -> String` (pure, unit-tested ring-buffer eviction logic extracted for testability).

- [ ] **Step 1: Add dependencies**

In `src-tauri/Cargo.toml`, add under `[dependencies]`:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt"] }
tracing-appender = "0.2"
```

- [ ] **Step 2: Write the failing test for ring-buffer eviction**

Create `src-tauri/src/logging.rs`:

```rust
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub const MAX_LINES: usize = 2000;

pub type LogBuffer = Arc<Mutex<VecDeque<String>>>;

/// Pushes a line into the buffer, evicting the oldest line first if at capacity.
/// Extracted as a pure function (buffer passed in) so eviction behavior is
/// unit-testable without spinning up a real tracing subscriber.
pub fn push_and_format(buffer: &LogBuffer, max_lines: usize, line: String) {
    let mut buf = buffer.lock().unwrap();
    if buf.len() >= max_lines {
        buf.pop_front();
    }
    buf.push_back(line);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushes_lines_in_order() {
        let buf: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
        push_and_format(&buf, 10, "a".into());
        push_and_format(&buf, 10, "b".into());
        assert_eq!(buf.lock().unwrap().iter().cloned().collect::<Vec<_>>(), vec!["a", "b"]);
    }

    #[test]
    fn evicts_oldest_when_at_capacity() {
        let buf: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
        push_and_format(&buf, 2, "a".into());
        push_and_format(&buf, 2, "b".into());
        push_and_format(&buf, 2, "c".into());
        assert_eq!(buf.lock().unwrap().iter().cloned().collect::<Vec<_>>(), vec!["b", "c"]);
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cd src-tauri && cargo test logging`
Expected: 2 tests pass.

- [ ] **Step 4: Implement the tracing `Layer` and `init()`**

Append to `src-tauri/src/logging.rs`:

```rust
use tauri::{AppHandle, Emitter};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;

struct EmitLayer {
    buffer: LogBuffer,
    app: AppHandle,
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }
}

impl<S> Layer<S> for EmitLayer
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let line = format!("[{}] {}", event.metadata().level(), visitor.message);
        push_and_format(&self.buffer, MAX_LINES, line.clone());
        let _ = self.app.emit("backend-log", line);
    }
}

/// Must be called from inside Tauri's `.setup()` closure — needs a live `AppHandle`
/// to emit live log events, which isn't available before the app is built.
pub fn init(app: &AppHandle, log_dir: std::path::PathBuf) -> LogBuffer {
    let buffer: LogBuffer = Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LINES)));
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::daily(&log_dir, "minive.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    // Leaked deliberately: the guard must live for the process lifetime to keep
    // flushing the non-blocking file writer; there's no natural owner to hold it.
    std::mem::forget(guard);
    let file_layer = tracing_subscriber::fmt::layer().with_writer(non_blocking).with_ansi(false);
    let emit_layer = EmitLayer { buffer: buffer.clone(), app: app.clone() };
    tracing_subscriber::registry().with(file_layer).with(emit_layer).init();
    buffer
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds cleanly. If the `Layer` trait bound doesn't match the installed `tracing-subscriber` version's exact signature, adjust per the compiler's suggested bound — the `for<'lookup> LookupSpan<'lookup>` bound is the standard one but exact wording can shift between minor versions.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/logging.rs
git commit -m "feat: tracing-based logging with ring buffer and live event emission"
```

---

### Task 2: Wire logging init into `lib.rs`, replace `eprintln!` call sites

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/registry.rs:60,63`

**Interfaces:**
- Consumes: `logging::init`, `logging::LogBuffer` (Task 1).
- Produces: `AppState.log_buffer: LogBuffer`, `get_backend_logs` command.

- [ ] **Step 1: Add `mod logging;` and `log_buffer` to `AppState`**

Add `mod logging;` to the top of `src-tauri/src/lib.rs` (with the other `mod` lines).

In `src-tauri/src/state.rs`, add the field:

```rust
pub log_buffer: crate::logging::LogBuffer,
```

- [ ] **Step 2: Initialize logging first thing inside `.setup()`**

In `src-tauri/src/lib.rs`, as the very first line inside the `.setup(|app| { ... })` closure (before the `update_handle` line, so the update-check task's `eprintln!` — which Step 4 converts to `tracing::warn!` — is captured):

```rust
let log_dir = app.path().app_data_dir().expect("app data dir").join("logs");
let log_buffer = logging::init(&app.handle(), log_dir);
```

Add `log_buffer` to the `app.manage(AppState { ... })` call:

```rust
app.manage(AppState {
    docker,
    registry: tokio::sync::Mutex::new(registry::Registry::load(reg_path)),
    settings: tokio::sync::Mutex::new(settings::Settings::load(settings_path)),
    catalog_cache_path,
    log_buffer,
    sessions: std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new())),
    next_session: AtomicU32::new(1),
});
```

(If `docs/superpowers/plans/2026-07-10-runtime-catalog.md` has not been implemented yet in this codebase, `AppState` won't have `settings`/`catalog_cache_path` — include only the fields that actually exist on `AppState` at the time this task runs; `docker`, `registry`, `log_buffer`, `sessions`, `next_session` are the baseline set.)

- [ ] **Step 3: Add the `get_backend_logs` command**

Append to `src-tauri/src/logging.rs`:

```rust
#[tauri::command]
pub fn get_backend_logs(state: tauri::State<'_, crate::state::AppState>) -> Vec<String> {
    state.log_buffer.lock().unwrap().iter().cloned().collect()
}
```

Register in `src-tauri/src/lib.rs`'s `tauri::generate_handler![...]`:

```rust
logging::get_backend_logs,
```

- [ ] **Step 4: Replace the three `eprintln!` call sites**

In `src-tauri/src/lib.rs:57`, change:
```rust
eprintln!("update check failed: {e}");
```
to:
```rust
tracing::warn!("update check failed: {e}");
```

In `src-tauri/src/registry.rs:60`, change:
```rust
Err(e) => { eprintln!("registry: serialize failed: {e}"); return; }
```
to:
```rust
Err(e) => { tracing::error!("registry: serialize failed: {e}"); return; }
```

In `src-tauri/src/registry.rs:63`, change:
```rust
eprintln!("registry: save failed: {e}");
```
to:
```rust
tracing::error!("registry: save failed: {e}");
```

- [ ] **Step 5: Verify it compiles and existing tests still pass**

Run: `cd src-tauri && cargo build && cargo test`
Expected: builds cleanly, all tests pass (the `registry.rs` tests don't touch these log lines directly, so no test changes needed).

- [ ] **Step 6: Manual verification**

Run: `npm run tauri dev`. In devtools console:
```js
await window.__TAURI__.core.invoke("get_backend_logs")
```
Expected: an array (possibly empty on a clean run with no warnings/errors yet — trigger one by stopping Docker Desktop and watching for a `docker-lost` related log line, or just confirm the call succeeds without error).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/state.rs src-tauri/src/registry.rs
git commit -m "feat: wire backend log buffer into app state, replace ad-hoc eprintln! calls"
```

---

### Task 3: Container logs command

**Files:**
- Create: `src-tauri/src/container_logs.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: Tauri command `stream_container_logs(name: String, on_output: Channel<String>) -> Result<(), String>`.

- [ ] **Step 1: Implement the module**

Create `src-tauri/src/container_logs.rs`:

```rust
use bollard::container::LogsOptions;
use futures_util::StreamExt;
use tauri::ipc::Channel;

use crate::env_manager::ctr_name;
use crate::state::AppState;

#[tauri::command]
pub async fn stream_container_logs(
    state: tauri::State<'_, AppState>,
    name: String,
    on_output: Channel<String>,
) -> Result<(), String> {
    let mut stream = state.docker.logs(
        &ctr_name(&name),
        Some(LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            tail: "200".into(),
            ..Default::default()
        }),
    );
    while let Some(item) = stream.next().await {
        let msg = item.map_err(|e| e.to_string())?;
        let _ = on_output.send(msg.to_string());
    }
    Ok(())
}
```

No unit test — same convention as other docker-touching commands in this codebase (needs a running daemon + container). Verified manually in Step 3.

Add `mod container_logs;` to `src-tauri/src/lib.rs`, and register the command in `tauri::generate_handler![...]`:

```rust
container_logs::stream_container_logs,
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds cleanly. If `LogOutput` (the stream item type) doesn't implement `.to_string()` in the installed bollard version, use `String::from_utf8_lossy(&msg.into_bytes()).into_owned()` instead, matching the pattern already used in `src-tauri/src/files.rs:82` for `exec_stream`.

- [ ] **Step 3: Manual verification**

Run `npm run tauri dev`, with at least one environment running. In devtools console:
```js
const out = new (window.__TAURI__.core.Channel)();
out.onmessage = console.log;
await window.__TAURI__.core.invoke("stream_container_logs", { name: "<an-existing-env-name>", onOutput: out });
```
Expected: log lines print live; compare against `docker logs -f minive-<name>` in a terminal.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/container_logs.rs src-tauri/src/lib.rs
git commit -m "feat: stream per-container docker logs"
```

---

### Task 4: Logs panel UI

**Files:**
- Create: `src/lib/LogsPanel.svelte`
- Modify: `src/lib/Home.svelte`

**Interfaces:**
- Consumes: Tauri commands `get_backend_logs`, `stream_container_logs` (Tasks 2, 3), `list_envs` (existing), Tauri event `backend-log` (Task 1).

- [ ] **Step 1: Create the component**

Create `src/lib/LogsPanel.svelte`:

```svelte
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
    const out = new Channel<string>();
    out.onmessage = (line) => { containerLines = [...containerLines.slice(-500), line]; };
    invoke("stream_container_logs", { name: selectedEnv, onOutput: out }).catch((e) => {
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
```

- [ ] **Step 2: Wire the entry point in `Home.svelte`**

In `src/lib/Home.svelte`, add the import:

```ts
import LogsPanel from "./LogsPanel.svelte";
```

Add state:

```ts
let showLogs = $state(false);
```

Add a button in the header's action group (alongside "Manage Images" / "+ New Environment" if `2026-07-10-runtime-catalog.md` has already been implemented, otherwise alongside "+ New Environment" directly):

```svelte
<button onclick={() => (showLogs = true)}>Logs</button>
```

Add the modal render:

```svelte
{#if showLogs}
  <LogsPanel onclose={() => (showLogs = false)} />
{/if}
```

- [ ] **Step 3: Verify the frontend builds and runs**

Run: `npm run build`
Expected: builds without TypeScript errors.

Run: `npm run tauri dev`, click "Logs".
Expected: App tab shows current buffer and updates live (trigger a log line by stopping/starting Docker Desktop to hit the `docker-lost`/`docker-back` paths, or wait for any warning); Container tab, with an env selected, streams that container's `docker logs -f` output.

- [ ] **Step 4: Commit**

```bash
git add src/lib/LogsPanel.svelte src/lib/Home.svelte
git commit -m "feat: logs panel for app and container logs"
```

---

### Task 5: Full verification pass

**Files:** none (verification only)

- [ ] **Step 1: Run the full backend test suite**

Run: `cd src-tauri && cargo test`
Expected: all tests pass, including the 2 new `logging` tests.

- [ ] **Step 2: Run the frontend build**

Run: `npm run build`
Expected: builds cleanly.

- [ ] **Step 3: Confirm no stray `eprintln!`/`println!` remain**

Run: `grep -rn "eprintln!\|println!" src-tauri/src`
Expected: no matches (all converted to `tracing::` calls in Task 2).

- [ ] **Step 4: End-to-end manual walkthrough**

Run: `npm run tauri dev`. Confirm: Logs panel opens from Home, App tab shows live-updating backend log lines, Container tab streams a running environment's docker output, and `app_data_dir()/logs/minive.log.<date>` exists on disk with the same content as the App tab.
