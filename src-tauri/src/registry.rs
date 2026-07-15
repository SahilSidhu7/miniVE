use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PortMap {
    pub host: u16,
    pub container: u16,
}

/// A user-managed shell script attached to an environment. `on_start`
/// scripts run automatically every time the container starts, so their
/// content should be idempotent (generated ones guard with `command -v`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptEntry {
    pub name: String,
    pub content: String,
    pub on_start: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EnvEntry {
    pub name: String,
    pub image: String,
    pub ports: Vec<PortMap>,
    #[serde(default)]
    pub scripts: Vec<ScriptEntry>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EnvStatus {
    Running,
    Stopped,
    Broken,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct EnvView {
    pub name: String,
    pub image: String,
    pub ports: Vec<PortMap>,
    pub status: EnvStatus,
}

pub struct DockerContainer {
    pub env_name: String,
    pub image: String,
    pub running: bool,
}

pub struct Registry {
    path: PathBuf,
    pub entries: Vec<EnvEntry>,
}

impl Registry {
    pub fn load(path: PathBuf) -> Self {
        let entries = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Registry { path, entries }
    }

    fn save(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("json.tmp");
        let json = match serde_json::to_string_pretty(&self.entries) {
            Ok(j) => j,
            Err(e) => { tracing::error!("registry: serialize failed: {e}"); return; }
        };
        if let Err(e) = std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &self.path)) {
            tracing::error!("registry: save failed: {e}");
        }
    }

    pub fn get(&self, name: &str) -> Option<&EnvEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    pub fn upsert(&mut self, entry: EnvEntry) {
        self.entries.retain(|e| e.name != entry.name);
        self.entries.push(entry);
        self.save();
    }

    pub fn remove(&mut self, name: &str) {
        self.entries.retain(|e| e.name != name);
        self.save();
    }

    pub fn upsert_script(&mut self, env: &str, script: ScriptEntry) -> Result<(), String> {
        let e = self.entries.iter_mut().find(|e| e.name == env).ok_or("no such environment")?;
        e.scripts.retain(|s| s.name != script.name);
        e.scripts.push(script);
        self.save();
        Ok(())
    }

    pub fn remove_script(&mut self, env: &str, script_name: &str) -> Result<(), String> {
        let e = self.entries.iter_mut().find(|e| e.name == env).ok_or("no such environment")?;
        e.scripts.retain(|s| s.name != script_name);
        self.save();
        Ok(())
    }

    pub fn reconcile(&mut self, containers: &[DockerContainer]) -> Vec<EnvView> {
        let mut adopted = false;
        for c in containers {
            if self.get(&c.env_name).is_none() {
                // ponytail: adopted envs lose port metadata; re-read from inspect if it ever matters
                self.entries.push(EnvEntry { name: c.env_name.clone(), image: c.image.clone(), ports: vec![], scripts: vec![] });
                adopted = true;
            }
        }
        if adopted {
            self.save();
        }
        self.entries
            .iter()
            .map(|e| {
                let status = match containers.iter().find(|c| c.env_name == e.name) {
                    Some(c) if c.running => EnvStatus::Running,
                    Some(_) => EnvStatus::Stopped,
                    None => EnvStatus::Broken,
                };
                EnvView { name: e.name.clone(), image: e.image.clone(), ports: e.ports.clone(), status }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("minive-reg-{}-{}.json", name, std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    fn entry(name: &str) -> EnvEntry {
        EnvEntry { name: name.into(), image: "python:3.12".into(), ports: vec![PortMap { host: 8000, container: 8000 }], scripts: vec![] }
    }

    #[test]
    fn load_missing_file_gives_empty_registry() {
        let r = Registry::load(tmp("missing"));
        assert!(r.entries.is_empty());
    }

    #[test]
    fn upsert_persists_across_reload() {
        let path = tmp("persist");
        let mut r = Registry::load(path.clone());
        r.upsert(entry("web"));
        let r2 = Registry::load(path);
        assert_eq!(r2.entries, vec![entry("web")]);
    }

    #[test]
    fn upsert_replaces_same_name() {
        let mut r = Registry::load(tmp("replace"));
        r.upsert(entry("web"));
        let mut changed = entry("web");
        changed.image = "node:20".into();
        r.upsert(changed.clone());
        assert_eq!(r.entries, vec![changed]);
    }

    #[test]
    fn remove_deletes_entry() {
        let mut r = Registry::load(tmp("remove"));
        r.upsert(entry("web"));
        r.remove("web");
        assert!(r.entries.is_empty());
    }

    #[test]
    fn reconcile_maps_statuses() {
        let mut r = Registry::load(tmp("status"));
        r.upsert(entry("running"));
        r.upsert(entry("stopped"));
        r.upsert(entry("gone"));
        let docker = vec![
            DockerContainer { env_name: "running".into(), image: "python:3.12".into(), running: true },
            DockerContainer { env_name: "stopped".into(), image: "python:3.12".into(), running: false },
        ];
        let views = r.reconcile(&docker);
        let status_of = |n: &str| views.iter().find(|v| v.name == n).unwrap().status.clone();
        assert_eq!(status_of("running"), EnvStatus::Running);
        assert_eq!(status_of("stopped"), EnvStatus::Stopped);
        assert_eq!(status_of("gone"), EnvStatus::Broken);
    }

    #[test]
    fn reconcile_adopts_unknown_labeled_containers() {
        let mut r = Registry::load(tmp("adopt"));
        let docker = vec![DockerContainer { env_name: "orphan".into(), image: "node:20".into(), running: true }];
        let views = r.reconcile(&docker);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].name, "orphan");
        assert_eq!(views[0].status, EnvStatus::Running);
        assert!(r.entries.iter().any(|e| e.name == "orphan"));
    }
}
