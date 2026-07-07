# Product Hunt launch kit

Launch Tue–Thu, 12:01 AM PT. Have the demo GIF as first gallery item — PH is
visual-first. Reply to every comment on launch day.

**Name:** miniVE

**Tagline (60 chars max):**
Disposable dev environments on your own machine

**Topics:** Developer Tools, Open Source, GitHub

**Description (260 chars max):**
Spin up isolated Python, Node, or Ubuntu environments in seconds — backed by
Docker on your own machine. Clone a repo, use a real terminal, preview web
ports. Delete it and every trace is gone. Free, MIT, no account, no cloud.

**First comment (from maker):**

Hey Product Hunt 👋

I built miniVE after realizing my laptop had three Python installs, four Node
versions, and a PATH I was afraid to look at — all from "just trying" other
people's repos.

miniVE gives you throwaway Linux dev environments on your own machine:

🐍 Pick a runtime — Python 3.10–3.12, Node 18–22, or blank Ubuntu
📂 Drop in files or clone a repo by URL
⌨️ Real multi-tab terminals; installs persist until *you* say otherwise
🌐 Map ports and preview web apps in-app
🗑️ Delete = truly gone. Container and volume removed; your machine stays clean.

What makes it different from cloud IDEs: there's no cloud. No account, no
telemetry, no subscription — it drives your local Docker, and everything it
creates is auditable with plain docker commands. MIT licensed.

Requires Docker Desktop. Windows, macOS, and Linux.

Would love your feedback — especially what's missing before this becomes your
default way to try untrusted code.

**Gallery captions (in order):**
1. Demo GIF — create → clone → run → preview → delete, in 30 seconds
2. Home screen — every environment at a glance, start/stop/delete
3. New environment wizard — runtime picker with versions and port mapping
4. Workspace — file tree, multi-tab terminal, live web preview
5. The receipt — `docker ps -a` empty after delete

**Alternatives positioning (if asked "how is this different from…"):**
- GitHub Codespaces / Gitpod: cloud, metered, account required. miniVE: local, free, none.
- Devcontainers: repo must ship config; editor-centric. miniVE: works on any repo, standalone app.
- Plain Docker CLI: same primitives, but you manage flags/volumes/cleanup. miniVE: two clicks and a guaranteed-clean delete.
