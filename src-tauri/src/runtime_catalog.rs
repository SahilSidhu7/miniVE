use regex::Regex;

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
