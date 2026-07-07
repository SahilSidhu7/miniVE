# Demo GIF shot list

One GIF, ~30–40 seconds, tells the whole story. Also capture 4 stills for
Product Hunt gallery / README while you're set up.

## Setup

- Recorder: ScreenToGif (free, Windows) — records straight to optimized GIF.
- Window: miniVE at ~1200×800. Hide taskbar clutter. 100% display scaling.
- Docker Desktop already running; pre-pull `python:3.12` beforehand so the GIF
  isn't 90% progress bar (`docker pull python:3.12`).
- Have this ready to paste: a small public repo with a web demo, e.g. your own
  test repo with `python -m http.server` content.
- Target file: `docs/assets/demo.gif` in the miniVE repo (README references it),
  copy also into landing repo if you want it on the site.

## Shots (keep moving — dead air kills GIFs)

1. **Home screen** (2s) — empty state or one existing env.
2. **New environment** (6s) — click New, type name `demo`, pick Python 3.12,
   add port 8000→8000, Create. Card appears, status → running.
3. **Clone** (5s) — open env, paste repo URL, clone; folder pops into file tree.
4. **Terminal** (8s) — open terminal tab, `ls`, then
   `python -m http.server 8000` — server starts.
5. **Preview** (6s) — preview pane renders the page. Beat for effect.
6. **Delete** (6s) — back to home, delete `demo`, confirm.
7. **The receipt** (5s) — a real terminal beside the app:
   `docker ps -a` → nothing. Freeze on that.

## Stills for gallery

- S1: home screen with 2–3 environments (mix of running/stopped)
- S2: new-environment wizard open
- S3: workspace — file tree + terminal + preview all visible
- S4: the empty `docker ps -a` receipt

## GIF settings

- 12–15 fps is plenty; keep under 10 MB (GitHub README inlines it).
- If over budget: drop to 10 fps and 1000px wide before cutting shots.
