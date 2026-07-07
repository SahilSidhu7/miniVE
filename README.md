# miniVE

miniVE is a desktop app that gives users lightweight, disposable, persistent Linux environments on their own machine. Users create an environment with a chosen runtime (Python, Node, or blank Ubuntu), put a project inside it (file upload or git clone), run and interact with it through a terminal, and preview web servers it exposes — all without polluting the host machine. Delete the environment and every trace is gone.

Environments run as Docker containers, so code inside them is genuinely isolated from the host.

## Prerequisites

- **Docker Desktop** (required) — download and install from [docker.com](https://www.docker.com/products/docker-desktop)
- Node.js 18+
- Rust toolchain (for building from source)

## Quick Start

```bash
npm install
npm run tauri dev
npm run tauri build
```

The first command installs dependencies. The second starts the dev server with hot reload. The third builds the production app.

## v1 Features

- **Create environment with chosen runtime** — Python 3.10/3.11/3.12, Node 18/20/22, or blank Ubuntu 24.04
- **Get files in** — select files or folders to upload into the environment, or clone a git repo by URL
- **Run and interact** — interactive terminal sessions inside the environment with live output
- **Web preview ports** — expose container ports to `localhost` and preview in the app

## Architecture

See [docs/superpowers/specs/2026-07-06-minive-design.md](docs/superpowers/specs/2026-07-06-minive-design.md) for the full architecture, including the Tauri 2 Rust backend, Svelte frontend, Docker integration, and component responsibilities.

## License

MIT. See [LICENSE](LICENSE) for details.

## Contributing

Issues and pull requests are welcome. Before submitting, run `cargo test` and `npm run build` to verify the changes.
