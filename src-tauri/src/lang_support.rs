//! Generates the per-environment language-support install script.
//!
//! Environments are now based on a Linux distro image; languages are
//! installed on top via this script. Every block is guarded with
//! `command -v`, so the script is cheap to re-run — it's saved as an
//! `on_start` script and executed on every container start, which makes
//! language support self-heal without re-paying install time.

use serde::Deserialize;

use crate::runtime_catalog::PkgManager;

#[derive(Deserialize, Clone, Debug)]
pub struct LangSpec {
    /// Runtime family key: "python", "node", "golang", "rust", "openjdk", "ruby", "php".
    pub key: String,
    /// Version from the catalog dropdown. Interpreted per language: exact for
    /// Go/Rust, major for Node/Java, advisory (distro version wins) for the rest.
    pub version: String,
}

fn major(version: &str) -> &str {
    version.split('.').next().unwrap_or(version)
}

/// One guarded install block per language, or None for unknown keys.
fn lang_block(spec: &LangSpec, mgr: PkgManager) -> Option<String> {
    let v = spec.version.trim();
    let m = major(v);
    let block = match (spec.key.as_str(), mgr) {
        ("python", PkgManager::Apt) => "command -v python3 >/dev/null 2>&1 || { apt-get update -qq; apt-get install -y -qq python3 python3-pip python3-venv; }".to_string(),
        ("python", PkgManager::Apk) => "command -v python3 >/dev/null 2>&1 || apk add --no-cache python3 py3-pip".to_string(),
        ("python", PkgManager::Dnf) => "command -v python3 >/dev/null 2>&1 || dnf install -y -q python3 python3-pip".to_string(),

        // NodeSource honors the requested major on apt/dnf; Alpine gets the distro's node.
        ("node", PkgManager::Apt) => format!(
            "command -v node >/dev/null 2>&1 || {{ apt-get update -qq; apt-get install -y -qq curl ca-certificates; curl -fsSL https://deb.nodesource.com/setup_{m}.x | bash -; apt-get install -y -qq nodejs; }}"
        ),
        ("node", PkgManager::Dnf) => format!(
            "command -v node >/dev/null 2>&1 || {{ curl -fsSL https://rpm.nodesource.com/setup_{m}.x | bash -; dnf install -y -q nodejs; }}"
        ),
        ("node", PkgManager::Apk) => "command -v node >/dev/null 2>&1 || apk add --no-cache nodejs npm".to_string(),

        // Official tarball: exact version, works on every distro (static toolchain).
        ("golang", _) => format!(
            "command -v go >/dev/null 2>&1 || [ -x /usr/local/go/bin/go ] || {{ curl -fsSL \"https://go.dev/dl/go{v}.linux-$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/').tar.gz\" | tar -xz -C /usr/local; echo 'export PATH=$PATH:/usr/local/go/bin' > /etc/profile.d/minive-go.sh; }}"
        ),

        // rustup: exact toolchain, all distros (detects musl on Alpine).
        ("rust", _) => format!(
            "command -v cargo >/dev/null 2>&1 || [ -x \"$HOME/.cargo/bin/cargo\" ] || {{ curl -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain {v} --profile minimal; echo '. $HOME/.cargo/env' > /etc/profile.d/minive-rust.sh; }}"
        ),

        ("openjdk", PkgManager::Apt) => format!("command -v java >/dev/null 2>&1 || {{ apt-get update -qq; apt-get install -y -qq openjdk-{m}-jdk || apt-get install -y -qq default-jdk; }}"),
        ("openjdk", PkgManager::Apk) => format!("command -v java >/dev/null 2>&1 || apk add --no-cache openjdk{m} || apk add --no-cache openjdk21"),
        ("openjdk", PkgManager::Dnf) => format!("command -v java >/dev/null 2>&1 || dnf install -y -q java-{m}-openjdk-devel || dnf install -y -q java-latest-openjdk-devel"),

        ("ruby", PkgManager::Apt) => "command -v ruby >/dev/null 2>&1 || { apt-get update -qq; apt-get install -y -qq ruby-full; }".to_string(),
        ("ruby", PkgManager::Apk) => "command -v ruby >/dev/null 2>&1 || apk add --no-cache ruby".to_string(),
        ("ruby", PkgManager::Dnf) => "command -v ruby >/dev/null 2>&1 || dnf install -y -q ruby".to_string(),

        ("php", PkgManager::Apt) => "command -v php >/dev/null 2>&1 || { apt-get update -qq; apt-get install -y -qq php-cli; }".to_string(),
        ("php", PkgManager::Apk) => "command -v php >/dev/null 2>&1 || apk add --no-cache php84-cli || apk add --no-cache php83-cli || apk add --no-cache php82-cli".to_string(),
        ("php", PkgManager::Dnf) => "command -v php >/dev/null 2>&1 || dnf install -y -q php-cli".to_string(),

        _ => return None,
    };
    Some(format!("echo '[minive] language: {} {v}'\n{block}", spec.key))
}

/// Full install script for the selected languages, or None if nothing to do.
/// Go/Rust/NodeSource blocks need curl, so a guarded curl install is prepended.
pub fn language_install_script(langs: &[LangSpec], mgr: PkgManager) -> Option<String> {
    if langs.is_empty() || mgr == PkgManager::None {
        return None;
    }
    let needs_curl = langs.iter().any(|l| matches!(l.key.as_str(), "golang" | "rust" | "node"));
    let mut parts = vec!["set -e".to_string()];
    if needs_curl {
        parts.push(curl_guard(mgr).to_string());
    }
    let blocks: Vec<String> = langs.iter().filter_map(|l| lang_block(l, mgr)).collect();
    if blocks.is_empty() {
        return None;
    }
    parts.extend(blocks);
    parts.push("echo '[minive] language support ready'".to_string());
    Some(parts.join("\n"))
}

fn curl_guard(mgr: PkgManager) -> &'static str {
    match mgr {
        PkgManager::Apt => "command -v curl >/dev/null 2>&1 || { apt-get update -qq; apt-get install -y -qq curl ca-certificates; }",
        PkgManager::Apk => "command -v curl >/dev/null 2>&1 || apk add --no-cache curl ca-certificates",
        PkgManager::Dnf => "command -v curl >/dev/null 2>&1 || dnf install -y -q curl ca-certificates",
        PkgManager::None => "",
    }
}

/// Installs the static Docker CLI so `docker` works inside the environment
/// (paired with a host docker.sock bind mount — see env_manager). Guarded, so
/// it's free after the first run.
pub fn docker_cli_install_script(mgr: PkgManager) -> String {
    format!(
        "set -e\n{}\ncommand -v docker >/dev/null 2>&1 || {{ echo '[minive] installing docker CLI...'; curl -fsSL \"https://download.docker.com/linux/static/stable/$(uname -m)/docker-27.5.1.tgz\" | tar -xz -C /tmp && mv /tmp/docker/docker /usr/local/bin/docker && rm -rf /tmp/docker; }}",
        curl_guard(mgr)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(key: &str, version: &str) -> LangSpec {
        LangSpec { key: key.into(), version: version.into() }
    }

    #[test]
    fn empty_selection_gives_none() {
        assert!(language_install_script(&[], PkgManager::Apt).is_none());
        assert!(language_install_script(&[spec("python", "3.12")], PkgManager::None).is_none());
    }

    #[test]
    fn node_apt_uses_nodesource_major() {
        let s = language_install_script(&[spec("node", "22.4.1")], PkgManager::Apt).unwrap();
        assert!(s.contains("deb.nodesource.com/setup_22.x"));
        assert!(s.contains("command -v curl"), "curl guard prepended");
    }

    #[test]
    fn go_uses_exact_version_tarball() {
        let s = language_install_script(&[spec("golang", "1.23.4")], PkgManager::Apk).unwrap();
        assert!(s.contains("go.dev/dl/go1.23.4.linux-"));
    }

    #[test]
    fn java_uses_major_package() {
        let s = language_install_script(&[spec("openjdk", "21")], PkgManager::Dnf).unwrap();
        assert!(s.contains("java-21-openjdk-devel"));
    }

    #[test]
    fn unknown_language_skipped_entirely() {
        assert!(language_install_script(&[spec("cobol", "85")], PkgManager::Apt).is_none());
    }

    #[test]
    fn multiple_languages_all_present() {
        let s = language_install_script(&[spec("python", "3.12"), spec("rust", "1.80.0")], PkgManager::Apt).unwrap();
        assert!(s.contains("python3"));
        assert!(s.contains("rustup"));
    }

    #[test]
    fn docker_cli_script_is_guarded() {
        let s = docker_cli_install_script(PkgManager::Apt);
        assert!(s.contains("command -v docker"));
        assert!(s.contains("download.docker.com/linux/static"));
    }
}
