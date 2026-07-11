<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="miniVE icon" width="96" />
</p>

<h1 align="center">miniVE</h1>

<p align="center">
  <b>Disposable Linux dev environments on your own machine.</b><br/>
  Spin up an isolated Python, Node, or Ubuntu box in seconds. Break it, trash it, delete it — your host stays clean.
</p>

<p align="center">
  <a href="https://github.com/SahilSidhu7/miniVE/releases/latest"><img src="https://img.shields.io/github/v/release/SahilSidhu7/miniVE?label=download" alt="Latest release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license"></a>
  <a href="https://sahilsidhu7.github.io/minive-landing/"><img src="https://img.shields.io/badge/website-minive-8A2BE2" alt="Website"></a>
</p>

<!-- demo GIF: docs/assets/demo.gif — see marketing/demo-shot-list.md -->

---

Ever needed to try a library, run someone else's repo, or test a script — without it touching your machine? miniVE gives you throwaway Linux environments backed by Docker containers:

- **Pick a runtime** — Python, Node, or Ubuntu, with versions fetched live from Docker Hub (works offline from cache too), not a hardcoded list
- **Start productive** — every environment comes with the basics (git, curl, build tools) plus optional package presets for your runtime, so you can clone and run immediately
- **Get code in** — upload files/folders or clone a git repo by URL
- **Work normally** — full interactive terminal (multi-tab), persistent between restarts
- **Preview web apps** — expose container ports and preview servers right in the app
- **See what's happening** — built-in Logs panel streams app logs and live container output
- **Control disk usage** — pin the runtime versions you use most (e.g. Ubuntu 22.04), prune cached images you don't
- **Delete = gone** — removing an environment removes every trace: container, volume, everything

## Install

> **Prerequisite:** [Docker Desktop](https://www.docker.com/products/docker-desktop/) must be installed and running. miniVE runs environments as Docker containers — that's what makes the isolation real.

Download the installer for your OS from the **[latest release](https://github.com/SahilSidhu7/miniVE/releases/latest)**, or use a package manager:

### Windows

```powershell
winget install SahilSidhu7.miniVE
```

Or grab the `.msi` from the [latest release](https://github.com/SahilSidhu7/miniVE/releases/latest). Windows SmartScreen may warn because the installer isn't code-signed yet — click **More info → Run anyway**.

### macOS

```bash
brew install --cask sahilsidhu7/tap/minive
```

Or download the `.dmg` (`aarch64` for Apple Silicon, `x64` for Intel). The app isn't notarized yet, so on first launch macOS will block it: open **System Settings → Privacy & Security**, scroll down, and click **Open Anyway** next to miniVE. This is a one-time step.

### Linux

Download the `.AppImage` (portable, `chmod +x` and run) or `.deb` from the [latest release](https://github.com/SahilSidhu7/miniVE/releases/latest).

The app checks for updates on launch and installs them with your confirmation.

## CLI

Prefer the terminal? `minive-cli` manages the same environments as the app — anything you create in one shows up in the other.

```bash
minive-cli create py --image python:3.12 --port 8000:8000   # create + start
minive-cli shell py        # drop into a shell inside it
minive-cli list            # see all environments
minive-cli stop py / start py / delete py
```

`create` defaults to `ubuntu:24.04` with the minimal preset (git + curl); pass `--preset none|minimal|full` to change it. Build it from source with `cargo build --release --bin minive-cli` in `src-tauri/` (binary lands in `src-tauri/target/release/`).

## How it works

Each environment is a Docker container with a persistent volume mounted at `/workspace`. The app talks to Docker directly — no daemon of its own, no cloud, no account. Your code never leaves your machine. Stopping an environment keeps its state (installed packages, files); deleting it removes the container **and** the volume, leaving nothing behind (`docker ps -a` / `docker volume ls` clean).

Full architecture: [docs/superpowers/specs/2026-07-06-minive-design.md](docs/superpowers/specs/2026-07-06-minive-design.md)

## Documentation

Guides, FAQ, and troubleshooting: **[docs](https://sahilsidhu7.github.io/minive-landing/docs.html)**

## Building from source

Requires Node.js 18+, Rust toolchain, and Docker Desktop.

```bash
npm install
npm run tauri dev     # dev with hot reload
npm run tauri build   # production installers
```

## Contributing

Issues and pull requests welcome. Before submitting, run `cargo test` and `npm run build`.

## License

MIT. See [LICENSE](LICENSE).
