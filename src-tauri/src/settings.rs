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
