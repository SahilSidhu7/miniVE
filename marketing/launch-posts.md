# Launch posts — ready to paste

Rules of thumb: post the HN one from your own account and stay in the comments for
the first 3–4 hours (that's where launches live or die). Post Reddit versions on
different days, not all at once. Never post the same text twice — each below is
tuned to its venue.

---

## Hacker News — Show HN

**Title:**
Show HN: miniVE – disposable Docker-backed dev environments in a desktop app

**Body:**

Hi HN. I built miniVE because I was tired of what "let me just try this repo"
does to a machine over a year: three Python installs, nvm-managed everything,
global packages I can't explain, and a PATH that reads like an archaeology dig.

miniVE is a small desktop app (Tauri, Rust + Svelte) that gives you disposable
Linux environments backed by Docker containers:

- Pick a runtime: Python 3.10–3.12, Node 18–22, or blank Ubuntu 24.04
- Drop files in or git-clone a repo straight into the environment
- Work in real multi-tab terminals; installs persist across restarts
- Map ports and preview web servers in the app
- Delete = container AND volume gone. `docker ps -a` shows nothing.

It intentionally does nothing you couldn't script with the Docker CLI — the
point is the two-click version, plus guaranteed cleanup. Everything it creates
carries a `minive.env` label so you can audit it with plain docker commands.

Free, MIT, no account, no telemetry. Windows/macOS/Linux.
Requires Docker Desktop (that's what makes the isolation real instead of pretend).

Code: https://github.com/SahilSidhu7/miniVE
Site: https://sahilsidhu7.github.io/minive-landing/

Happy to answer anything about the Tauri + bollard (Docker API) plumbing —
the terminal resize/reconnect handling was the hairiest part.

**Prepared answers for likely comments:**

- *"Why not devcontainers?"* — Devcontainers are editor-centric and config-file-driven
  (great for teams standardizing a repo). miniVE is for the other case: someone else's
  random repo, a quick experiment, no `.devcontainer` folder, no VS Code required.
- *"Why not just docker run -it?"* — You can! miniVE adds: persistent named volumes,
  file upload UI, port preview, terminal tabs, and delete-means-delete cleanup with
  zero flags to remember. It's a convenience layer, not a new primitive.
- *"Docker Desktop licensing?"* — Free for individuals/small business; on Linux, plain
  Docker Engine works, no Desktop needed.
- *"Unsigned binaries?"* — Yes for now; signing certs cost money the project doesn't
  have yet. Updates are minisign-verified by the Tauri updater regardless.

---

## Reddit — r/docker

**Title:** I built a desktop app that turns Docker containers into disposable dev environments (MIT, no account)

**Body:**

Docker folks — I kept using containers as throwaway dev boxes (`docker run -it
python:3.12 bash`, volume here, port flag there) and finally wrapped the workflow
in a small desktop app.

miniVE: pick Python/Node/Ubuntu, it creates a labeled container + volume mounted at
`/workspace`. Terminal tabs in the app, file upload, git clone by URL, port mapping
with a preview pane. Delete removes container **and** volume — the whole pitch is that
`docker ps -a` and `docker volume ls` stay clean.

Nothing you couldn't do with the CLI (everything is inspectable via the `minive.env`
label) — it's the two-click version for when you don't want to remember flags.

Free + MIT: https://github.com/SahilSidhu7/miniVE

Would genuinely like this sub's take: what would you need before using something
like this daily — compose support? resource limits per env? SSH agent forwarding?

---

## Reddit — r/selfhosted

**Title:** miniVE — self-hosted disposable dev environments (your machine, your Docker, no cloud)

**Body:**

The cloud-IDE crowd solved "try code without wrecking your machine" by renting you
a VM. miniVE is the local version: a free desktop app that creates disposable
Linux environments as Docker containers on your own hardware.

- No account, no telemetry, no server component — talks only to your local Docker daemon
- Python / Node / Ubuntu runtimes, real terminal, git clone, web port preview
- Everything's a labeled container + volume you can audit with plain docker commands
- Delete an env and every trace is gone

MIT licensed: https://github.com/SahilSidhu7/miniVE

---

## Reddit — r/webdev

**Title:** Stop installing three versions of Node to try one repo — I built a free tool for throwaway dev environments

**Body:**

You know the drill: repo needs Node 18, you're on 22, nvm shuffle, global installs,
and six months later your machine is a museum of half-finished experiments.

I built miniVE — a free desktop app that spins up disposable dev environments
(Node 18/20/22, Python 3.10–3.12, or plain Ubuntu) as Docker containers:

1. New environment → pick runtime → map port 3000
2. Clone the repo by URL (git runs inside the env, not your machine)
3. `npm install && npm run dev` in the built-in terminal
4. Preview pane shows the dev server; or open localhost:3000 in your browser
5. Done judging the repo? Delete. Your machine never knew it happened.

Free, MIT, works on Win/Mac/Linux, needs Docker Desktop.
https://github.com/SahilSidhu7/miniVE

---

## X / Twitter thread

1/ Your machine remembers every experiment. Three Pythons, four Nodes, a PATH
that reads like a crime scene.

I built miniVE to fix that: disposable dev environments on your own machine.
Free, open source. 🧵

2/ Pick a runtime (Python 3.10–3.12, Node 18–22, or blank Ubuntu). miniVE spins
up an isolated Docker-backed environment in seconds.

Clone a repo into it. Open a terminal. Install anything.

Your host machine: untouched.

3/ Run a dev server inside and preview it right in the app — real port mapping
to localhost, no tunnels, no cloud.

[attach demo GIF]

4/ The best part is leaving. Delete an environment and the container AND its
volume are gone. `docker ps -a`: nothing. Like it never happened.

5/ No account. No telemetry. No cloud. MIT licensed. Talks only to your local
Docker daemon — you can audit everything it makes with plain docker commands.

Windows / macOS / Linux 👇
https://github.com/SahilSidhu7/miniVE

---

## dev.to article

**Title:** Run any repo without wrecking your machine: disposable dev environments with miniVE

**Tags:** docker, opensource, productivity, tooling

**Body:**

### The problem

Every developer's machine is a graveyard of experiments. That repo you tried
once installed a global CLI. That tutorial needed Python 3.10 when you had 3.12.
That "quick test" edited your PATH. Individually harmless; collectively, the
reason "works on my machine" is a meme.

The industry's answers each have a catch. Cloud IDEs put your code on someone
else's computer and meter it. Devcontainers are excellent *if* the repo ships the
config and you live in VS Code. Raw `docker run` works but makes you the flag
manager, volume janitor, and port accountant.

### The tool

miniVE is a small open-source desktop app (Tauri: Rust + Svelte) that wraps the
docker-as-throwaway-dev-box workflow in a UI:

**Create** — name it, pick Python 3.10/3.11/3.12, Node 18/20/22, or blank
Ubuntu 24.04, optionally map ports. It creates one container plus one persistent
volume mounted at `/workspace`.

**Work** — multi-tab interactive terminals (a real TTY: colors, Ctrl-C, TUI apps
work). Upload files or paste a git URL to clone inside the environment — git runs
in the container, not on your host. `pip install` / `npm install` persist across
stop/start.

**Preview** — run `python -m http.server 8000` or a dev server bound to
`0.0.0.0`, and the mapped port renders in a preview pane (or your browser at
localhost).

**Delete** — the container and its volume are removed together.
`docker ps -a | grep minive` → nothing. That guarantee is the entire point.

### What it deliberately isn't

There's no magic. Every environment is one container + one volume with a
`minive.env` label; audit it with plain `docker` commands. No account, no
telemetry, no server. If you're happy scripting the Docker CLI, miniVE does
nothing you can't — it's the two-click version with guaranteed cleanup.

### Try it

Free, MIT, Windows/macOS/Linux. Requires Docker Desktop (Docker Engine is
enough on Linux).

→ https://github.com/SahilSidhu7/miniVE

If you try it, I want the bug reports and the feature complaints — especially:
what would it take for this to replace your current scratch-environment habit?
