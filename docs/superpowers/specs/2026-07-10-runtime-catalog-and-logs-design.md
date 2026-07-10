# Runtime Catalog, Package Presets, Image Cache Management & Logs Panel

Status: approved for planning
Date: 2026-07-10

## Problem

Today `RUNTIMES` is a hardcoded 7-entry array (`src/lib/types.ts:5-13`): Python 3.10-3.12, Node 18-22, Ubuntu 24.04. New environments boot to a bare base image with nothing installed — not even `git` — so the only thing a user can do post-create is clone a repo (if they picked that option in the wizard) or install things themselves by hand. There's no way to see which images are already pulled locally, no way to prune them, and no way to favorite frequently-used versions. Separately, the Rust backend has no structured logging (just a handful of `eprintln!` calls in `registry.rs` and `lib.rs`) and no way to view container logs from the app, making it hard to debug either the app itself or a misbehaving environment.

## Goals

1. Wizard offers live version lists (pulled from Docker Hub) instead of a hardcoded array, across a wider set of runtime families.
2. New environments can optionally get a package preset installed on first boot.
3. User can pin favorite versions and manage (view/prune) locally-pulled images.
4. Dev can view backend app logs and per-container docker logs from inside the app.

## Non-goals

- Auto-discovering arbitrary Docker Hub repos — the set of runtime families is a curated static table, not user-extensible in v1.
- Editing/creating custom package presets (free-text apt lists) — v1 ships fixed presets only.
- This spec does not cover the UI overhaul (collapsible sidebar, multi-terminal layout) — that's a separate design.

## Architecture

### Runtime catalog (backend)

New `src-tauri/src/runtime_catalog.rs`:

```rust
enum PkgManager { Apt, Apk, Dnf, None }

struct RuntimeFamily {
    key: &'static str,          // "python"
    display_name: &'static str, // "Python"
    docker_repo: &'static str,  // "python" (Docker Hub library/ namespace)
    pkg_manager: PkgManager,
    tag_pattern: &'static str,  // regex to keep clean version tags, drop -rc/-alpine/-slim dupes
}
```

Static table covers: python, node, ubuntu, debian, alpine, fedora, golang, rust, openjdk, ruby, php.

- `fetch_tags(repo: &str) -> Vec<String>` calls Docker Hub's public v2 API: `GET https://hub.docker.com/v2/repositories/library/{repo}/tags?page_size=100`, filters via `tag_pattern`, sorts descending (newest first).
- Results cached to disk (`runtime_catalog_cache.json`, same dir as `registry.json`) with a 24h TTL to avoid re-hitting Docker Hub on every wizard open.
- On fetch failure (offline, rate-limited, API change): serve the last on-disk cache; if no cache exists yet, fall back to the current hardcoded 7-entry list so the wizard is never empty.
- New Tauri command `list_runtime_catalog() -> Vec<RuntimeFamilyWithVersions>`.

### Settings (pins)

Extend persisted state with a new `Settings` struct (own file, same atomic tmp-rename save pattern as `Registry::save()` in `registry.rs:53-65`):

```rust
struct Settings {
    pinned_versions: Vec<String>, // e.g. "python:3.11", "ubuntu:22.04"
}
```

Loaded/saved next to `Registry` in `state.rs`/`lib.rs`. Pinned versions sort to the top of each family's list in the wizard dropdown.

### Image cache management

- `list_cached_images() -> Vec<CachedImage>` — wraps `docker images` (bollard) returning repo:tag, size, created date.
- `remove_cached_image(id: String)` — wraps `docker rmi`.
- New frontend screen (Settings → "Manage Images" tab): table of locally-pulled images with size + pin toggle + delete button.

### Package presets

Wizard gets a third field alongside runtime/version/clone-url: preset `None | Minimal | Full`.

- Minimal: `git`, `curl`.
- Full: `git`, `curl`, `vim`, `unzip`, + build tools (`build-essential` on apt, `build-base` on apk, `"Development Tools"` group on dnf).

After container create+start (`env_manager.rs::create_env`, right after the container start call ~line 118-120), if preset ≠ None: one `docker exec` running the install command matching the family's declared `pkg_manager`. Families with `pkg_manager: None` skip silently (no guessing at package managers). Install failure is non-fatal — the environment is still created and usable, a warning surfaces in the wizard log stream (same `Channel<String>` already used for pull progress).

### Logs panel

No structured logging exists today — replace the `eprintln!` calls in `registry.rs:60/63` and `lib.rs:57` as part of this work.

**Backend app logs:**
- Add `tracing` + `tracing-subscriber` with a custom `Layer` that pushes formatted lines into an in-memory ring buffer (`Mutex<VecDeque<String>>`, cap ~2000 lines) and also writes to a rotating file under `app_data_dir()/logs/`.
- `get_backend_logs() -> Vec<String>` returns the current buffer; a `backend-log` event (Tauri emit) pushes new lines live, same pattern as existing progress Channels.

**Container logs:**
- `stream_container_logs(env_id: String) -> Channel<String>` using bollard's `logs()` API (bollard already a dependency, used elsewhere for exec/attach).

**Frontend:**
- New "Logs" panel (tab or bottom drawer, not modal — should stay open alongside terminal work): toggle between "App" and "Container" (env picker when toggled to Container, if more than one env is open).

## Data flow

1. Wizard mounts → calls `list_runtime_catalog()` → backend serves cache or fetches live → returns families+versions+pinned-first ordering.
2. User picks runtime/version/preset/ports/clone-url → submit → `create_env` (existing flow) → after container start, if preset set, `docker exec` install step → existing `clone_repo` step if git URL set.
3. Settings → Manage Images → `list_cached_images()` on mount, `remove_cached_image(id)` on delete click, pin toggle writes to `Settings.pinned_versions` and saves.
4. Logs panel → `get_backend_logs()` on open + subscribe to `backend-log` events; switching to Container mode calls `stream_container_logs(env_id)`.

## Error handling

- Docker Hub unreachable/rate-limited: never blocks the wizard — cache or hardcoded fallback list used, no user-facing error unless both are unavailable (shouldn't happen given the hardcoded fallback).
- Package preset install failure: non-fatal, warning shown in wizard log stream, environment remains usable.
- Log file write failure: swallowed (logging must never crash the app) — ring buffer / live event still works even if file write fails.

## Testing

- Rust unit tests (`src-tauri/tests`): tag-filter regex per family (valid tags kept, `-rc`/junk dropped), cache TTL fallback logic (stale cache → refetch, fetch failure → serve stale cache, no cache + failure → hardcoded fallback).
- Rust unit test: package-manager install command builder per `PkgManager` variant produces the expected command string.
- Manual verification: wizard shows live versions, pin/unpin re-orders list, image prune removes from `docker images`, logs panel shows both app and container output live.
