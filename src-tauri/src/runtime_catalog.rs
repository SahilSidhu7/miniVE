use regex::Regex;
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PkgManager {
    Apt,
    Apk,
    Dnf,
    None,
}

#[allow(dead_code)]
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
