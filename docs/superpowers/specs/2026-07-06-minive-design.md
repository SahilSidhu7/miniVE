# miniVE — Design Spec

**Date:** 2026-07-06
**Status:** Approved

## Purpose

miniVE is a desktop app that gives users lightweight, disposable, persistent Linux environments on their own machine. Users create an environment with a chosen runtime (Python, Node, or blank Ubuntu), put a project inside it (file upload or git clone), run and interact with it through a terminal, and preview web servers it exposes — all without polluting the host machine. Delete the environment and every trace is gone.

Environments run as Docker containers, so code inside them is genuinely isolated from the host.

## Decisions Made

| Decision | Choice | Rationale |
|---|---|---|
| Form factor | Desktop app | Full machine access, native UX |
| Shell tech | Tauri 2 (Rust backend) | Hardware-efficient, small footprint, strong Rust ecosystem for Docker (bollard), PTY (portable-pty), and future AI features (llama.cpp / candle bindings) |
| Isolation | Container-backed (Docker) | Real isolation; safe to run untrusted code. Docker Desktop is an accepted hard prerequisite |
| Env model | Long-lived container per environment (Approach A) | Customizations (apt/pip/npm installs) persist; feels like a real VM |
| Frontend | Svelte + xterm.js in Tauri WebView | Light, fast, terminal-ready |

## v1 Scope

Must-have:

1. **Create environment with chosen runtime** — Python 3.10/3.11/3.12, Node 18/20/22, or blank Ubuntu 24.04.
2. **Get files in** — drag-drop upload into the environment, or clone a git repo by URL.
3. **Run + terminal + logs** — interactive terminal sessions inside the environment; live output.
4. **Web preview ports** — expose container ports to `localhost`; preview pane in the app.

Out of scope for v1: LLM-specific tooling, GPU passthrough, environment templates/sharing, multi-machine, container mode without Docker.

## Architecture

```
┌─────────────────────────── miniVE (Tauri 2 app) ───────────────────────────┐
│  Frontend (WebView)                Rust backend (Tauri core)               │
│  ┌───────────────────┐  invoke/   ┌──────────────────────────┐             │
│  │ Svelte UI          │  events    │ env_manager  (bollard)   │──► Docker  │
│  │ - env list/cards   │◄──────────►│ term_bridge  (exec+PTY)  │    Engine  │
│  │ - xterm.js terminal│            │ files        (upload/git)│    (npipe) │
│  │ - port preview     │            │ registry     (env state) │             │
│  └───────────────────┘            └──────────────────────────┘             │
└─────────────────────────────────────────────────────────────────────────────┘
        Each environment = 1 Docker container + 1 named volume (/workspace)
```

### Environment model

- **Environment = one Docker container + one named volume** mounted at `/workspace`.
- Create env → pull runtime image → create volume → start container with `sleep infinity` as PID 1.
- Stop/start/delete an env = container lifecycle operations. Delete removes container **and** volume.
- Containers are labeled `minive.env=<name>` so state is recoverable from Docker alone.
- Docker Engine is reached over the Windows named pipe (`npipe`) via the bollard crate.

### Backend components (Rust)

| Component | Responsibility | Depends on |
|---|---|---|
| `env_manager` | Env CRUD: pull image, create/start/stop/remove container + volume, port bindings | bollard |
| `term_bridge` | Terminal sessions: `docker exec` with TTY, stream stdin/stdout between xterm.js (Tauri events) and the exec socket. Multiple sessions per env | bollard exec API |
| `files` | Upload: tar the dropped files, `put_archive` into `/workspace`. Clone: run `git clone <url>` inside the container (no host git needed). Read-only file tree listing for the UI | bollard, tar |
| `registry` | Persistent env metadata (name, image, ports, container id) in one JSON file in the app data dir. Reconciles with Docker labels at startup | serde_json |

Docker itself is the source of truth for container status; the registry stores only what Docker can't (display names, chosen ports before first start).

### Frontend (Svelte)

1. **Home screen** — env cards showing name, runtime icon, running/stopped status, ports. Actions: start/stop, open, delete. "+ New Environment" button.
2. **New env wizard** — name → runtime picker → optional port mappings → optional git URL → Create (with image pull progress).
3. **Env workspace** — left: file tree of `/workspace` (read + upload); center: terminal tabs (xterm.js); right/bottom: port preview pane (iframe on `localhost:<port>`) + open-in-browser button.

### Startup flow

1. Check Docker reachable over npipe.
2. Not reachable → full-screen guidance: install/start Docker Desktop, link, re-check button. App unusable until Docker is up (honest hard prerequisite).
3. Reachable → reconcile registry against `minive.env`-labeled containers, render home screen, subscribe to Docker events stream for live status.

## Error Handling

| Failure | Behavior |
|---|---|
| Docker daemon dies mid-session | Global banner "Docker not running" + retry; env cards grey out; no crash |
| Image pull fails (offline / bad tag) | Inline wizard error; env not created; no orphan container or volume |
| Container dies unexpectedly | Docker events stream flips card to "stopped"; open terminal tabs show "session ended" + restart button |
| Port already in use on host | Start fails with toast naming the port; env stays stopped; user edits port mapping |
| Registry/Docker drift | Startup reconcile: labeled containers missing from JSON are re-adopted; JSON entries with no container are marked "broken" with a delete option |
| Upload failure / cancel | Streamed with progress; cancel supported; partial upload discarded |

## Testing

- **Rust unit tests** — registry logic (reconcile, CRUD) against a mocked Docker trait.
- **Rust integration test** — behind `--ignored`, requires real Docker: create env → exec `echo hi` → verify output → destroy env and volume.
- **Frontend** — no component test suite in v1 (thin UI over invoke calls). One smoke check: app boots and the Docker-missing screen renders when the daemon is absent.
- **Manual release checklist** (kept in repo): create Python env, create Node env, clone a repo, run a dev server, preview the port, delete both envs, verify no leftover containers/volumes.

## Future (explicitly deferred)

- LLM runtimes (llama.cpp images, GPU passthrough)
- Environment templates and export/import
- Resource limits (CPU/RAM caps per env)
- Linux/macOS support (bollard socket path change; mostly free later)
