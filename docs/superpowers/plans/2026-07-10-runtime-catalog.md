# Runtime Catalog, Package Presets & Image Cache Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the hardcoded 7-entry `RUNTIMES` list with a live Docker Hub-backed catalog across 11 runtime families, add optional package presets on environment creation, and let users pin favorite versions and manage locally-cached images.

**Architecture:** New Rust module `runtime_catalog.rs` holds a static table of runtime families (repo name + package manager) and fetches version tags from Docker Hub's public v2 API, disk-cached with a 24h TTL and a hardcoded fallback. A new `settings.rs` persists pinned versions using the same atomic-write pattern as `registry.rs`. A new `images.rs` wraps `docker images`/`docker rmi` for cache management. `env_manager.rs::create_env` gains an optional post-start package-install step reusing the existing `exec_stream` helper from `files.rs`. Frontend replaces the static `RUNTIMES` const with a fetched catalog, adds a preset dropdown to the wizard, and adds a new "Manage Images" modal.

**Tech Stack:** Rust (Tauri 2, bollard 0.18, tokio), reqwest (new dep) for HTTP, regex (new dep) for tag filtering, Svelte 5 (runes) frontend.

## Global Constraints

- Docker Hub fetch failures must never block the wizard — always fall back to cache, then to a hardcoded list (spec: `docs/superpowers/specs/2026-07-10-runtime-catalog-and-logs-design.md`, "Error handling").
- Package preset install failures are non-fatal — the environment is still created and usable (same spec section).
- Curated static family table only — no arbitrary Docker Hub repo discovery in v1 (spec: "Non-goals").
- Follow existing error convention: all Tauri commands return `Result<T, String>`, errors via `.map_err(|e| e.to_string())`.

---

### Task 1: Runtime family table + version tag filtering/sorting (pure, unit-tested)

**Files:**
- Create: `src-tauri/src/runtime_catalog.rs`
- Modify: `src-tauri/Cargo.toml` (add `regex` dependency)

**Interfaces:**
- Produces: `pub struct RuntimeFamily { key: &'static str, display_name: &'static str, docker_repo: &'static str, pkg_manager: PkgManager }`, `pub const FAMILIES: &[RuntimeFamily]`, `pub enum PkgManager { Apt, Apk, Dnf, None }`, `pub fn select_versions(raw_tags: Vec<String>) -> Vec<String>` (filters to clean version tags, sorted newest-first).

- [ ] **Step 1: Add `regex` dependency**

In `src-tauri/Cargo.toml`, add under `[dependencies]`:

```toml
regex = "1"
```

- [ ] **Step 2: Write the failing tests**

Create `src-tauri/src/runtime_catalog.rs`:

```rust
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PkgManager {
    Apt,
    Apk,
    Dnf,
    None,
}

pub struct RuntimeFamily {
    pub key: &'static str,
    pub display_name: &'static str,
    pub docker_repo: &'static str,
    pub pkg_manager: PkgManager,
}

pub const FAMILIES: &[RuntimeFamily] = &[
    RuntimeFamily { key: "python", display_name: "Python", docker_repo: "python", pkg_manager: PkgManager::Apt },
    RuntimeFamily { key: "node", display_name: "Node", docker_repo: "node", pkg_manager: PkgManager::Apt },
    RuntimeFamily { key: "ubuntu", display_name: "Ubuntu", docker_repo: "ubuntu", pkg_manager: PkgManager::Apt },
    RuntimeFamily { key: "debian", display_name: "Debian", docker_repo: "debian", pkg_manager: PkgManager::Apt },
    RuntimeFamily { key: "alpine", display_name: "Alpine", docker_repo: "alpine", pkg_manager: PkgManager::Apk },
    RuntimeFamily { key: "fedora", display_name: "Fedora", docker_repo: "fedora", pkg_manager: PkgManager::Dnf },
    RuntimeFamily { key: "golang", display_name: "Go", docker_repo: "golang", pkg_manager: PkgManager::Apt },
    RuntimeFamily { key: "rust", display_name: "Rust", docker_repo: "rust", pkg_manager: PkgManager::Apt },
    RuntimeFamily { key: "openjdk", display_name: "Java (OpenJDK)", docker_repo: "openjdk", pkg_manager: PkgManager::Apt },
    RuntimeFamily { key: "ruby", display_name: "Ruby", docker_repo: "ruby", pkg_manager: PkgManager::Apt },
    RuntimeFamily { key: "php", display_name: "PHP", docker_repo: "php", pkg_manager: PkgManager::Apt },
];

pub fn pkg_manager_for_image(image: &str) -> PkgManager {
    let repo = image.split(':').next().unwrap_or(image);
    FAMILIES.iter().find(|f| f.docker_repo == repo).map(|f| f.pkg_manager).unwrap_or(PkgManager::None)
}

fn version_tag_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    // Keep only plain numeric versions (X, X.Y, X.Y.Z) — drops "latest", "-rc1",
    // "-slim", "-alpine", distro-codename suffixes, etc. One shared pattern
    // covers every family cleanly; ponytail: no per-family regex needed.
    RE.get_or_init(|| Regex::new(r"^[0-9]+(\.[0-9]+){0,2}$").unwrap())
}

fn parse_version(tag: &str) -> Vec<u32> {
    tag.split('.').map(|p| p.parse::<u32>().unwrap_or(0)).collect()
}

/// Filters raw Docker Hub tag names down to clean version strings, sorted newest-first.
pub fn select_versions(raw_tags: Vec<String>) -> Vec<String> {
    let re = version_tag_regex();
    let mut versions: Vec<String> = raw_tags.into_iter().filter(|t| re.is_match(t)).collect();
    versions.sort_by(|a, b| parse_version(b).cmp(&parse_version(a)));
    versions.dedup();
    versions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_and_sorts_version_tags() {
        let raw = vec![
            "3.12".to_string(),
            "3.12-slim".to_string(),
            "latest".to_string(),
            "3.9".to_string(),
            "3.13.0rc1".to_string(),
            "3.10".to_string(),
        ];
        assert_eq!(select_versions(raw), vec!["3.12", "3.10", "3.9"]);
    }

    #[test]
    fn dedupes_identical_versions() {
        let raw = vec!["22".to_string(), "22".to_string(), "20".to_string()];
        assert_eq!(select_versions(raw), vec!["22", "20"]);
    }

    #[test]
    fn empty_input_gives_empty_output() {
        assert!(select_versions(vec![]).is_empty());
    }

    #[test]
    fn pkg_manager_lookup_matches_repo_before_colon() {
        assert_eq!(pkg_manager_for_image("python:3.12"), PkgManager::Apt);
        assert_eq!(pkg_manager_for_image("alpine:3.19"), PkgManager::Apk);
        assert_eq!(pkg_manager_for_image("fedora:40"), PkgManager::Dnf);
    }

    #[test]
    fn pkg_manager_unknown_image_is_none() {
        assert_eq!(pkg_manager_for_image("some/random-image:1.0"), PkgManager::None);
    }
}
```

Add `mod runtime_catalog;` to `src-tauri/src/lib.rs` (near the other `mod` declarations at the top) so this compiles.

- [ ] **Step 3: Run tests to verify they pass**

Run: `cd src-tauri && cargo test runtime_catalog`
Expected: 5 tests pass (`filters_and_sorts_version_tags`, `dedupes_identical_versions`, `empty_input_gives_empty_output`, `pkg_manager_lookup_matches_repo_before_colon`, `pkg_manager_unknown_image_is_none`).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/runtime_catalog.rs src-tauri/src/lib.rs
git commit -m "feat: runtime family table and version tag filtering"
```

---

### Task 2: Docker Hub fetch + disk cache with TTL and fallback

**Files:**
- Modify: `src-tauri/src/runtime_catalog.rs`
- Modify: `src-tauri/Cargo.toml` (add `reqwest` dependency)

**Interfaces:**
- Consumes: `FAMILIES`, `select_versions` (Task 1).
- Produces: `pub struct FamilyVersions { pub key: String, pub display_name: String, pub versions: Vec<String> }`, `pub fn is_stale(fetched_at_unix: u64, now_unix: u64) -> bool`, `pub fn load_cache(path: &Path) -> Option<CatalogCache>`, `pub fn save_cache(path: &Path, cache: &CatalogCache)`, `pub fn fallback_catalog() -> Vec<FamilyVersions>`, `pub async fn fetch_all_families() -> Result<Vec<FamilyVersions>, String>`.

- [ ] **Step 1: Add `reqwest` dependency**

In `src-tauri/Cargo.toml`, add under `[dependencies]` (rustls avoids pulling in a platform OpenSSL toolchain for cross-platform release builds):

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

- [ ] **Step 2: Write the failing tests for cache TTL and fallback**

Append to `src-tauri/src/runtime_catalog.rs`:

```rust
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_TTL_SECS: u64 = 24 * 60 * 60;

// camelCase so the frontend's `FamilyVersions` type (`displayName`) matches the
// wire format directly — see the mirrored `#[serde(rename_all = "camelCase")]`
// note on `CachedImage` in images.rs (Task 6) for the same reasoning.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FamilyVersions {
    pub key: String,
    pub display_name: String,
    pub versions: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CatalogCache {
    pub fetched_at_unix: u64,
    pub families: Vec<FamilyVersions>,
}

pub fn is_stale(fetched_at_unix: u64, now_unix: u64) -> bool {
    now_unix.saturating_sub(fetched_at_unix) > CACHE_TTL_SECS
}

pub fn load_cache(path: &Path) -> Option<CatalogCache> {
    let s = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&s).ok()
}

pub fn save_cache(path: &Path, cache: &CatalogCache) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("json.tmp");
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, path));
    }
}

/// The pre-catalog hardcoded list — used when there is no cache yet AND
/// Docker Hub is unreachable, so the wizard is never empty on first run offline.
pub fn fallback_catalog() -> Vec<FamilyVersions> {
    vec![
        FamilyVersions { key: "python".into(), display_name: "Python".into(), versions: vec!["3.12".into(), "3.11".into(), "3.10".into()] },
        FamilyVersions { key: "node".into(), display_name: "Node".into(), versions: vec!["22".into(), "20".into(), "18".into()] },
        FamilyVersions { key: "ubuntu".into(), display_name: "Ubuntu".into(), versions: vec!["24.04".into()] },
    ]
}

#[derive(Deserialize)]
struct DockerHubTagsResponse {
    results: Vec<DockerHubTag>,
}

#[derive(Deserialize)]
struct DockerHubTag {
    name: String,
}

async fn fetch_family_versions(client: &reqwest::Client, family: &RuntimeFamily) -> Result<Vec<String>, String> {
    // First page only (page_size=100, sorted by last-updated) — a v1 limitation:
    // very old versions past the first page won't appear. Paginate if that's a complaint.
    let url = format!(
        "https://hub.docker.com/v2/repositories/library/{}/tags?page_size=100&ordering=last_updated",
        family.docker_repo
    );
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let parsed: DockerHubTagsResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(select_versions(parsed.results.into_iter().map(|t| t.name).collect()))
}

/// Best-effort across all families: a single family failing doesn't fail the
/// whole fetch. Only errors out if every family failed (e.g. fully offline).
pub async fn fetch_all_families() -> Result<Vec<FamilyVersions>, String> {
    let client = reqwest::Client::new();
    let mut out = Vec::new();
    for family in FAMILIES {
        match fetch_family_versions(&client, family).await {
            Ok(versions) if !versions.is_empty() => out.push(FamilyVersions {
                key: family.key.to_string(),
                display_name: family.display_name.to_string(),
                versions,
            }),
            Ok(_) => {}
            Err(e) => eprintln!("runtime_catalog: fetch failed for {}: {e}", family.docker_repo),
        }
    }
    if out.is_empty() {
        Err("all runtime family fetches failed".into())
    } else {
        Ok(out)
    }
}

pub fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("minive-catalog-{}-{}.json", name, std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn stale_when_older_than_ttl() {
        assert!(is_stale(1000, 1000 + CACHE_TTL_SECS + 1));
        assert!(!is_stale(1000, 1000 + CACHE_TTL_SECS - 1));
    }

    #[test]
    fn load_missing_cache_gives_none() {
        assert!(load_cache(&tmp("missing")).is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = tmp("roundtrip");
        let cache = CatalogCache {
            fetched_at_unix: 12345,
            families: vec![FamilyVersions { key: "python".into(), display_name: "Python".into(), versions: vec!["3.12".into()] }],
        };
        save_cache(&path, &cache);
        let loaded = load_cache(&path).unwrap();
        assert_eq!(loaded.fetched_at_unix, 12345);
        assert_eq!(loaded.families.len(), 1);
        assert_eq!(loaded.families[0].key, "python");
    }

    #[test]
    fn fallback_catalog_matches_original_hardcoded_list() {
        let fb = fallback_catalog();
        assert_eq!(fb.len(), 3);
        assert_eq!(fb[0].key, "python");
        assert_eq!(fb[0].versions, vec!["3.12", "3.11", "3.10"]);
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cd src-tauri && cargo test runtime_catalog`
Expected: previous 5 tests plus 4 new cache tests all pass (9 total). `fetch_all_families`/`fetch_family_versions` are not unit tested (they need network) — verified manually in Task 4's integration check.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/runtime_catalog.rs
git commit -m "feat: Docker Hub tag fetch with disk cache, TTL, and offline fallback"
```

---

### Task 3: Package preset install command builder (pure, unit-tested)

**Files:**
- Modify: `src-tauri/src/runtime_catalog.rs`

**Interfaces:**
- Consumes: `PkgManager` (Task 1).
- Produces: `pub enum PackagePreset { None, Minimal, Full }` (Deserialize, Clone, Copy, PartialEq), `pub fn install_command(preset: PackagePreset, mgr: PkgManager) -> Option<Vec<String>>`.

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/src/runtime_catalog.rs`:

```rust
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PackagePreset {
    None,
    Minimal,
    Full,
}

/// Returns a `["sh", "-c", "<script>"]` exec command, or `None` if there's
/// nothing to install (preset is None, or the image's family has no known
/// package manager — no guessing).
pub fn install_command(preset: PackagePreset, mgr: PkgManager) -> Option<Vec<String>> {
    if preset == PackagePreset::None || mgr == PkgManager::None {
        return None;
    }
    let packages: &[&str] = match (preset, mgr) {
        (PackagePreset::Minimal, _) => &["git", "curl"],
        (PackagePreset::Full, PkgManager::Apt) => &["git", "curl", "vim", "unzip", "build-essential"],
        (PackagePreset::Full, PkgManager::Apk) => &["git", "curl", "vim", "unzip", "build-base"],
        (PackagePreset::Full, PkgManager::Dnf) => &["git", "curl", "vim", "unzip", "@development-tools"],
        (PackagePreset::Full, PkgManager::None) | (PackagePreset::None, _) => return None,
    };
    let pkg_list = packages.join(" ");
    let script = match mgr {
        PkgManager::Apt => format!("apt-get update -qq && apt-get install -y -qq {pkg_list}"),
        PkgManager::Apk => format!("apk add --no-cache {pkg_list}"),
        PkgManager::Dnf => format!("dnf install -y -q {pkg_list}"),
        PkgManager::None => return None,
    };
    Some(vec!["sh".into(), "-c".into(), script])
}

#[cfg(test)]
mod preset_tests {
    use super::*;

    #[test]
    fn minimal_preset_same_packages_regardless_of_manager() {
        let cmd = install_command(PackagePreset::Minimal, PkgManager::Apt).unwrap();
        assert_eq!(cmd, vec!["sh", "-c", "apt-get update -qq && apt-get install -y -qq git curl"]);
    }

    #[test]
    fn full_preset_apk_uses_build_base() {
        let cmd = install_command(PackagePreset::Full, PkgManager::Apk).unwrap();
        assert!(cmd[2].starts_with("apk add --no-cache"));
        assert!(cmd[2].contains("build-base"));
    }

    #[test]
    fn full_preset_dnf_uses_development_tools_group() {
        let cmd = install_command(PackagePreset::Full, PkgManager::Dnf).unwrap();
        assert!(cmd[2].starts_with("dnf install -y -q"));
        assert!(cmd[2].contains("@development-tools"));
    }

    #[test]
    fn none_preset_returns_none() {
        assert!(install_command(PackagePreset::None, PkgManager::Apt).is_none());
    }

    #[test]
    fn unknown_pkg_manager_returns_none_even_with_preset() {
        assert!(install_command(PackagePreset::Full, PkgManager::None).is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cd src-tauri && cargo test runtime_catalog`
Expected: 14 tests total pass across `runtime_catalog`.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/runtime_catalog.rs
git commit -m "feat: package preset install command builder"
```

---

### Task 4: Settings (pinned versions) persistence

**Files:**
- Create: `src-tauri/src/settings.rs`

**Interfaces:**
- Produces: `pub struct Settings { pub pinned_versions: Vec<String> }` with `load(path) -> Self`, `pin(&mut self, version: String)`, `unpin(&mut self, version: &str)`.

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/src/settings.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct SettingsData {
    pinned_versions: Vec<String>,
}

pub struct Settings {
    path: PathBuf,
    data: SettingsData,
}

impl Settings {
    pub fn load(path: PathBuf) -> Self {
        let data = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Settings { path, data }
    }

    fn save(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("json.tmp");
        if let Ok(json) = serde_json::to_string_pretty(&self.data) {
            let _ = std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &self.path));
        }
    }

    pub fn pinned_versions(&self) -> Vec<String> {
        self.data.pinned_versions.clone()
    }

    pub fn pin(&mut self, version: String) {
        if !self.data.pinned_versions.contains(&version) {
            self.data.pinned_versions.push(version);
            self.save();
        }
    }

    pub fn unpin(&mut self, version: &str) {
        self.data.pinned_versions.retain(|v| v != version);
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("minive-settings-{}-{}.json", name, std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn load_missing_file_gives_empty_settings() {
        let s = Settings::load(tmp("missing"));
        assert!(s.pinned_versions().is_empty());
    }

    #[test]
    fn pin_persists_across_reload() {
        let path = tmp("persist");
        let mut s = Settings::load(path.clone());
        s.pin("python:3.11".into());
        let s2 = Settings::load(path);
        assert_eq!(s2.pinned_versions(), vec!["python:3.11".to_string()]);
    }

    #[test]
    fn pin_is_idempotent() {
        let mut s = Settings::load(tmp("idempotent"));
        s.pin("node:20".into());
        s.pin("node:20".into());
        assert_eq!(s.pinned_versions(), vec!["node:20".to_string()]);
    }

    #[test]
    fn unpin_removes_entry() {
        let mut s = Settings::load(tmp("unpin"));
        s.pin("ubuntu:24.04".into());
        s.unpin("ubuntu:24.04");
        assert!(s.pinned_versions().is_empty());
    }
}
```

Add `mod settings;` to `src-tauri/src/lib.rs`.

- [ ] **Step 2: Run tests to verify they pass**

Run: `cd src-tauri && cargo test settings`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/settings.rs src-tauri/src/lib.rs
git commit -m "feat: persisted settings for pinned runtime versions"
```

---

### Task 5: Wire catalog + settings into AppState and Tauri commands

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/runtime_catalog.rs` (add the `list_runtime_catalog` command)
- Modify: `src-tauri/src/settings.rs` (add pin/unpin/list commands)

**Interfaces:**
- Consumes: `Settings::load/pin/unpin/pinned_versions` (Task 4), `is_stale`, `load_cache`, `save_cache`, `fallback_catalog`, `fetch_all_families`, `now_unix` (Task 2).
- Produces: Tauri commands `list_runtime_catalog`, `pin_version`, `unpin_version`, `list_pinned_versions` registered in `invoke_handler!`.

- [ ] **Step 1: Add fields to `AppState`**

In `src-tauri/src/state.rs`, add imports and fields:

```rust
use std::path::PathBuf;
use crate::settings::Settings;
```

Add to the `AppState` struct:

```rust
pub struct AppState {
    pub docker: bollard::Docker,
    pub registry: Mutex<Registry>,
    pub settings: Mutex<Settings>,
    pub catalog_cache_path: PathBuf,
    pub sessions: Arc<Mutex<HashMap<u32, Arc<Mutex<Session>>>>>,
    pub next_session: AtomicU32,
}
```

- [ ] **Step 2: Wire construction in `lib.rs` setup**

In `src-tauri/src/lib.rs`, inside the `.setup(|app| { ... })` closure, near where `reg_path` is built (around line 62-66), add:

```rust
let settings_path = app.path().app_data_dir().expect("app data dir").join("settings.json");
let catalog_cache_path = app.path().app_data_dir().expect("app data dir").join("runtime_catalog_cache.json");
```

Update the `app.manage(AppState { ... })` call to include the new fields:

```rust
app.manage(AppState {
    docker,
    registry: tokio::sync::Mutex::new(registry::Registry::load(reg_path)),
    settings: tokio::sync::Mutex::new(settings::Settings::load(settings_path)),
    catalog_cache_path,
    sessions: std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new())),
    next_session: AtomicU32::new(1),
});
```

- [ ] **Step 3: Add the `list_runtime_catalog` command**

Append to `src-tauri/src/runtime_catalog.rs`:

```rust
#[tauri::command]
pub async fn list_runtime_catalog(state: tauri::State<'_, crate::state::AppState>) -> Result<Vec<FamilyVersions>, String> {
    let now = now_unix();
    if let Some(cache) = load_cache(&state.catalog_cache_path) {
        if !is_stale(cache.fetched_at_unix, now) {
            return Ok(cache.families);
        }
    }
    match fetch_all_families().await {
        Ok(families) => {
            save_cache(&state.catalog_cache_path, &CatalogCache { fetched_at_unix: now, families: families.clone() });
            Ok(families)
        }
        Err(_) => {
            if let Some(cache) = load_cache(&state.catalog_cache_path) {
                Ok(cache.families)
            } else {
                Ok(fallback_catalog())
            }
        }
    }
}
```

- [ ] **Step 4: Add pin/unpin/list commands**

Append to `src-tauri/src/settings.rs`:

```rust
#[tauri::command]
pub async fn pin_version(state: tauri::State<'_, crate::state::AppState>, version: String) -> Result<(), String> {
    state.settings.lock().await.pin(version);
    Ok(())
}

#[tauri::command]
pub async fn unpin_version(state: tauri::State<'_, crate::state::AppState>, version: String) -> Result<(), String> {
    state.settings.lock().await.unpin(&version);
    Ok(())
}

#[tauri::command]
pub async fn list_pinned_versions(state: tauri::State<'_, crate::state::AppState>) -> Result<Vec<String>, String> {
    Ok(state.settings.lock().await.pinned_versions())
}
```

- [ ] **Step 5: Register commands in `invoke_handler!`**

In `src-tauri/src/lib.rs`, add to the `tauri::generate_handler![...]` list (after `env_manager::delete_env`):

```rust
runtime_catalog::list_runtime_catalog,
settings::pin_version,
settings::unpin_version,
settings::list_pinned_versions,
```

- [ ] **Step 6: Verify it compiles and existing tests still pass**

Run: `cd src-tauri && cargo build && cargo test`
Expected: builds cleanly, all prior tests (registry, runtime_catalog, settings) still pass.

- [ ] **Step 7: Manual verification**

Run: `npm run tauri dev`. Open the browser devtools console in the dev window and run:
```js
await window.__TAURI__.core.invoke("list_runtime_catalog")
```
Expected: an array of `{key, displayName?, versions}`-shaped objects (11 families, or fewer if some Docker Hub fetches failed) — either freshly fetched or the fallback list, never an error.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/src/lib.rs src-tauri/src/runtime_catalog.rs src-tauri/src/settings.rs
git commit -m "feat: wire runtime catalog and pinned-version settings into Tauri commands"
```

---

### Task 6: Image cache management commands

**Files:**
- Create: `src-tauri/src/images.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `pub struct CachedImage { id, repo_tag, size_bytes, created_unix }` (Serialize), Tauri commands `list_cached_images`, `remove_cached_image`.

- [ ] **Step 1: Implement the module**

Create `src-tauri/src/images.rs`:

```rust
use bollard::image::{ListImagesOptions, RemoveImageOptions};
use serde::Serialize;

use crate::state::AppState;

// camelCase so `ManageImages.svelte` can read `img.repoTag`/`sizeBytes`/`createdUnix`
// directly — plain serde field names would serialize as snake_case instead.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedImage {
    pub id: String,
    pub repo_tag: String,
    pub size_bytes: i64,
    pub created_unix: i64,
}

#[tauri::command]
pub async fn list_cached_images(state: tauri::State<'_, AppState>) -> Result<Vec<CachedImage>, String> {
    let images = state
        .docker
        .list_images(None::<ListImagesOptions<String>>)
        .await
        .map_err(|e| e.to_string())?;
    Ok(images
        .into_iter()
        .map(|img| CachedImage {
            id: img.id.clone(),
            repo_tag: img.repo_tags.first().cloned().unwrap_or(img.id),
            size_bytes: img.size,
            created_unix: img.created,
        })
        .collect())
}

#[tauri::command]
pub async fn remove_cached_image(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .docker
        .remove_image(&id, None::<RemoveImageOptions>, None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

No unit tests here — same convention as `env_manager.rs`'s docker-touching commands, which aren't unit tested either (bollard calls need a running daemon). Verified manually in Step 3 below.

Add `mod images;` to `src-tauri/src/lib.rs`, and register both commands in `tauri::generate_handler![...]`:

```rust
images::list_cached_images,
images::remove_cached_image,
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds cleanly. Fix any bollard API signature drift (field/method names) reported by the compiler — `ImageSummary` and `Docker::remove_image` signatures are current as of bollard 0.18 but double-check against `cargo doc --open -p bollard` if the build fails here.

- [ ] **Step 3: Manual verification**

Run: `npm run tauri dev`, then in devtools console:
```js
await window.__TAURI__.core.invoke("list_cached_images")
```
Expected: array reflecting `docker images` output on the local machine (cross-check with `docker images` in a terminal).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/images.rs src-tauri/src/lib.rs
git commit -m "feat: list and remove locally-cached Docker images"
```

---

### Task 7: Package preset install step in `create_env`

**Files:**
- Modify: `src-tauri/src/files.rs:62` (make `exec_stream` visible to `env_manager`)
- Modify: `src-tauri/src/env_manager.rs`

**Interfaces:**
- Consumes: `crate::files::exec_stream` (now `pub(crate)`), `crate::runtime_catalog::{PackagePreset, pkg_manager_for_image, install_command}` (Tasks 1, 3).
- Produces: `CreateEnvSpec.preset: PackagePreset` field.

- [ ] **Step 1: Make `exec_stream` crate-visible**

In `src-tauri/src/files.rs:62`, change:

```rust
async fn exec_stream(
```
to:
```rust
pub(crate) async fn exec_stream(
```

- [ ] **Step 2: Add `preset` to `CreateEnvSpec` and run the install step**

In `src-tauri/src/env_manager.rs`, add the import at the top (near the other `crate::` imports at line 12):

```rust
use crate::runtime_catalog::{install_command, pkg_manager_for_image, PackagePreset};
```

Modify `CreateEnvSpec` (lines 34-39) to add the field:

```rust
#[derive(Deserialize)]
pub struct CreateEnvSpec {
    pub name: String,
    pub image: String,
    pub ports: Vec<PortMap>,
    pub preset: PackagePreset,
}
```

In `create_env`, right after the container-start block succeeds and before building `entry` (i.e. right after line 151's closing `}` for the `start_container` error handling, before line 153's `let entry = ...`), insert:

```rust
    // 4. Optional package preset — non-fatal: the env is already created and
    // usable even if this install fails, so we only warn on the progress stream.
    if let Some(cmd) = install_command(spec.preset, pkg_manager_for_image(&spec.image)) {
        if let Err(e) = crate::files::exec_stream(&state, &ctr_name(&spec.name), cmd, &on_progress).await {
            let _ = on_progress.send(format!("[minive] package preset install failed (non-fatal): {e}"));
        }
    }
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds cleanly.

- [ ] **Step 4: Manual verification**

Run `npm run tauri dev`, create an environment via the current UI (Task 8 will add the preset dropdown — for now, manually pass `preset: "minimal"` from devtools):
```js
const progress = new (window.__TAURI__.core.Channel)();
progress.onmessage = console.log;
await window.__TAURI__.core.invoke("create_env", { spec: { name: "preset-test", image: "python:3.12", ports: [], preset: "minimal" }, onProgress: progress });
```
Expected: progress log includes the `apt-get install` output, and `docker exec minive-preset-test which git curl` (run in a real terminal) succeeds for both. Clean up with `invoke("delete_env", { name: "preset-test" })`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/files.rs src-tauri/src/env_manager.rs
git commit -m "feat: install package preset on environment creation"
```

---

### Task 8: Frontend catalog store

**Files:**
- Modify: `src/lib/types.ts`
- Create: `src/lib/catalog.ts`

**Interfaces:**
- Consumes: Tauri commands `list_runtime_catalog`, `list_pinned_versions` (Task 5).
- Produces: `export type FamilyVersions`, `export type PackagePreset`, `export async function loadCatalog(): Promise<FamilyVersions[]>`, `export function runtimeLabel(image: string): string`, `export function catalogEntries(): { label: string; image: string }[]` (pinned-first), `export async function loadPinned(): Promise<string[]>`, `export function isPinned(image: string): boolean`.

- [ ] **Step 1: Replace the hardcoded `RUNTIMES` const in `types.ts`**

Replace the full contents of `src/lib/types.ts` with:

```ts
export type PortMap = { host: number; container: number };
export type EnvStatus = "running" | "stopped" | "broken";
export type EnvView = { name: string; image: string; ports: PortMap[]; status: EnvStatus };

export type FamilyVersions = { key: string; displayName: string; versions: string[] };
export type PackagePreset = "none" | "minimal" | "full";
```

(`runtimeLabel` moves to `catalog.ts` since it now needs fetched state, not just a static table.)

- [ ] **Step 2: Create the catalog module**

Create `src/lib/catalog.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { FamilyVersions } from "./types";

let families: FamilyVersions[] = [];
let pinned: string[] = [];

export async function loadCatalog(): Promise<FamilyVersions[]> {
  families = await invoke<FamilyVersions[]>("list_runtime_catalog");
  return families;
}

export async function loadPinned(): Promise<string[]> {
  pinned = await invoke<string[]>("list_pinned_versions");
  return pinned;
}

export function isPinned(image: string): boolean {
  return pinned.includes(image);
}

export async function pinVersion(image: string): Promise<void> {
  await invoke("pin_version", { version: image });
  pinned = [...pinned, image];
}

export async function unpinVersion(image: string): Promise<void> {
  await invoke("unpin_version", { version: image });
  pinned = pinned.filter((v) => v !== image);
}

export function runtimeLabel(image: string): string {
  const [repo, tag] = image.split(":");
  const family = families.find((f) => f.key === repo);
  return family ? `${family.displayName} ${tag}` : image;
}

/** Flat {label, image} list for the wizard dropdown, pinned versions sorted first. */
export function catalogEntries(): { label: string; image: string }[] {
  const entries: { label: string; image: string }[] = [];
  for (const f of families) {
    for (const v of f.versions) {
      entries.push({ label: `${f.displayName} ${v}`, image: `${f.key}:${v}` });
    }
  }
  entries.sort((a, b) => Number(isPinned(b.image)) - Number(isPinned(a.image)));
  return entries;
}
```

- [ ] **Step 3: Find and update other `RUNTIMES`/`runtimeLabel` import sites**

Run: `grep -rn "RUNTIMES\|runtimeLabel" src/lib src/routes`
Expected output (before this task, from earlier exploration): `src/lib/Wizard.svelte:3` (`RUNTIMES`), `src/lib/Home.svelte:6` (`runtimeLabel`). Task 9 updates `Wizard.svelte`; for `Home.svelte`, change the import at line 6 from:
```ts
import { runtimeLabel } from "./types";
```
to:
```ts
import { runtimeLabel } from "./catalog";
```

- [ ] **Step 4: Verify the frontend builds**

Run: `npm run build`
Expected: builds without TypeScript errors (Wizard.svelte will still reference the old `RUNTIMES` import until Task 9 — if this task is executed standalone, expect a build error there that Task 9 resolves; run `npx svelte-check` to confirm only `Wizard.svelte` is flagged).

- [ ] **Step 5: Commit**

```bash
git add src/lib/types.ts src/lib/catalog.ts src/lib/Home.svelte
git commit -m "feat: frontend runtime catalog store replacing hardcoded RUNTIMES"
```

---

### Task 9: Wizard preset dropdown + live catalog

**Files:**
- Modify: `src/lib/Wizard.svelte`

**Interfaces:**
- Consumes: `loadCatalog`, `loadPinned`, `catalogEntries` (Task 8), `PackagePreset` type (Task 8).

- [ ] **Step 1: Update the script block**

In `src/lib/Wizard.svelte`, replace lines 1-37 with:

```svelte
<script lang="ts">
  import { invoke, Channel } from "@tauri-apps/api/core";
  import type { PortMap, PackagePreset } from "./types";
  import { loadCatalog, loadPinned, catalogEntries } from "./catalog";
  import { onMount } from "svelte";

  let { onclose, oncreated }: { onclose: () => void; oncreated: (name: string) => void } = $props();

  let entries: { label: string; image: string }[] = $state([]);
  let name = $state("");
  let image = $state("");
  let preset: PackagePreset = $state("minimal");
  let ports: PortMap[] = $state([]);
  let gitUrl = $state("");
  let busy = $state(false);
  let log: string[] = $state([]);
  let error = $state("");

  onMount(async () => {
    await Promise.all([loadCatalog(), loadPinned()]);
    entries = catalogEntries();
    if (entries.length) image = entries[0].image;
  });

  function addPort() { ports = [...ports, { host: 8000, container: 8000 }]; }
  function removePort(i: number) { ports = ports.filter((_, idx) => idx !== i); }

  async function create() {
    busy = true;
    error = "";
    log = [];
    const progress = new Channel<string>();
    progress.onmessage = (line) => { log = [...log.slice(-200), line]; };
    try {
      await invoke("create_env", { spec: { name, image, ports, preset }, onProgress: progress });
      if (gitUrl.trim()) {
        const out = new Channel<string>();
        out.onmessage = (line) => { log = [...log.slice(-200), line]; };
        const code = await invoke<number>("clone_repo", { name, url: gitUrl.trim(), onOutput: out });
        if (code !== 0) { error = `git clone exited with code ${code}`; busy = false; return; }
      }
      oncreated(name);
    } catch (e) {
      error = String(e);
      busy = false;
    }
  }
</script>
```

- [ ] **Step 2: Update the markup**

Replace the `<label>Runtime ...</label>` block (lines 44-48 of the original) with:

```svelte
    <label>Runtime
      <select bind:value={image} disabled={busy}>
        {#each entries as e}<option value={e.image}>{e.label}</option>{/each}
      </select>
    </label>
    <label>Packages
      <select bind:value={preset} disabled={busy}>
        <option value="none">None</option>
        <option value="minimal">Minimal (git, curl)</option>
        <option value="full">Full (+ vim, unzip, build tools)</option>
      </select>
    </label>
```

- [ ] **Step 3: Verify the frontend builds and runs**

Run: `npm run build`
Expected: builds without TypeScript errors.

Run: `npm run tauri dev`, open the wizard.
Expected: Runtime dropdown populated from the live catalog (or fallback if offline), a Packages dropdown defaulting to "Minimal", environment creation still works end-to-end including the git clone path.

- [ ] **Step 4: Commit**

```bash
git add src/lib/Wizard.svelte
git commit -m "feat: wizard uses live runtime catalog and package presets"
```

---

### Task 10: Manage Images screen

**Files:**
- Create: `src/lib/ManageImages.svelte`
- Modify: `src/lib/Home.svelte`

**Interfaces:**
- Consumes: Tauri commands `list_cached_images`, `remove_cached_image` (Task 6), `isPinned`, `pinVersion`, `unpinVersion`, `loadPinned` (Task 8).

- [ ] **Step 1: Create the component**

Create `src/lib/ManageImages.svelte`:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { isPinned, pinVersion, unpinVersion, loadPinned } from "./catalog";

  let { onclose }: { onclose: () => void } = $props();

  type CachedImage = { id: string; repoTag: string; sizeBytes: number; createdUnix: number };
  let images: CachedImage[] = $state([]);
  let error = $state("");
  let busyId = $state("");

  function formatSize(bytes: number): string {
    const mb = bytes / (1024 * 1024);
    return mb > 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(0)} MB`;
  }

  async function refresh() {
    try {
      images = await invoke<CachedImage[]>("list_cached_images");
      await loadPinned();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(id: string) {
    busyId = id;
    try {
      await invoke("remove_cached_image", { id });
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busyId = "";
    }
  }

  async function togglePin(repoTag: string) {
    if (isPinned(repoTag)) await unpinVersion(repoTag);
    else await pinVersion(repoTag);
    images = [...images]; // re-render pin state
  }

  onMount(refresh);
</script>

<div class="overlay">
  <div class="modal">
    <h2>Manage Images</h2>
    {#if error}<p class="error">{error}</p>{/if}
    <table>
      <thead><tr><th>Image</th><th>Size</th><th>Pin</th><th></th></tr></thead>
      <tbody>
        {#each images as img (img.id)}
          <tr>
            <td>{img.repoTag}</td>
            <td>{formatSize(img.sizeBytes)}</td>
            <td><button onclick={() => togglePin(img.repoTag)}>{isPinned(img.repoTag) ? "★" : "☆"}</button></td>
            <td><button disabled={busyId === img.id} onclick={() => remove(img.id)}>{busyId === img.id ? "…" : "Delete"}</button></td>
          </tr>
        {:else}
          <tr><td colspan="4">No cached images.</td></tr>
        {/each}
      </tbody>
    </table>
    <div class="actions">
      <button onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: grid; place-items: center; }
  .modal { background: #1e1e1e; border-radius: 8px; padding: 1.5rem; width: 36rem; max-height: 85vh; overflow-y: auto; display: grid; gap: 0.75rem; }
  table { width: 100%; border-collapse: collapse; }
  th, td { text-align: left; padding: 0.35rem 0.5rem; border-bottom: 1px solid #333; }
  .error { color: #f87171; }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
</style>
```

- [ ] **Step 2: Wire the entry point in `Home.svelte`**

In `src/lib/Home.svelte`, add the import (near line 7):

```ts
import ManageImages from "./ManageImages.svelte";
```

Add state (near line 11):

```ts
let showImages = $state(false);
```

Update the header (lines 40-43) to add the button:

```svelte
  <header>
    <h1>miniVE</h1>
    <div class="header-actions">
      <button onclick={() => (showImages = true)}>Manage Images</button>
      <button onclick={() => (showWizard = true)}>+ New Environment</button>
    </div>
  </header>
```

Add the modal render after the existing `{#if showWizard}` block (after line 77):

```svelte
{#if showImages}
  <ManageImages onclose={() => (showImages = false)} />
{/if}
```

Add a `.header-actions { display: flex; gap: 0.5rem; }` rule to the `<style>` block.

- [ ] **Step 3: Verify the frontend builds and runs**

Run: `npm run build`
Expected: builds without TypeScript errors.

Run: `npm run tauri dev`, click "Manage Images".
Expected: table lists locally-pulled images matching `docker images` in a terminal; pin toggle persists (check `settings.json` in the app data dir); delete removes the image (verify with `docker images` afterward).

- [ ] **Step 4: Commit**

```bash
git add src/lib/ManageImages.svelte src/lib/Home.svelte
git commit -m "feat: manage images screen for pinning and pruning cached runtimes"
```

---

### Task 11: Full verification pass

**Files:** none (verification only)

- [ ] **Step 1: Run the full backend test suite**

Run: `cd src-tauri && cargo test`
Expected: all tests pass (registry: 6, runtime_catalog: 14, settings: 4, plus any pre-existing tests in `src-tauri/tests`).

- [ ] **Step 2: Run the frontend build**

Run: `npm run build`
Expected: builds cleanly.

- [ ] **Step 3: End-to-end manual walkthrough**

Run: `npm run tauri dev`. Walk through:
1. Open wizard — confirm runtime dropdown shows more than the original 7 entries (or the 3-family fallback if offline).
2. Create an env with preset "Full" — confirm `git`, `curl`, `vim`, and a compiler (`gcc`/`cc`) are present via a terminal tab in the new env.
3. Open Manage Images — pin a version, confirm it now sorts first in a fresh wizard open; delete an unused image, confirm it disappears from `docker images`.
4. Restart the app — confirm pinned versions and the catalog cache both survive (catalog served from cache without a new Docker Hub hit within 24h — check no network tab activity to hub.docker.com on the second load, e.g. via browser devtools Network tab in the dev window).

- [ ] **Step 4: Update README if the "no packages on start" limitation was mentioned**

Run: `grep -n "clone a git repo\|upload files" README.md`
If found, no change needed — the pitch text already matches the new behavior (presets are opt-in, default stays close to today's minimal experience). Skip if nothing needs updating.
