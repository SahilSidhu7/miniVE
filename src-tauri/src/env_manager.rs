use std::collections::HashMap;

use bollard::container::{Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions, StartContainerOptions};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, PortBinding};
use bollard::volume::CreateVolumeOptions;
use bollard::Docker;
use futures_util::StreamExt;
use serde::Deserialize;
use tauri::ipc::Channel;

use crate::registry::{DockerContainer, EnvEntry, EnvStatus, EnvView, PortMap};
use crate::runtime_catalog::{install_command, pkg_manager_for_image, PackagePreset};
use crate::state::AppState;

pub const LABEL: &str = "minive.env";

pub fn ctr_name(env: &str) -> String {
    format!("minive-{env}")
}

pub fn vol_name(env: &str) -> String {
    format!("minive-{env}-ws")
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    name.len() <= 31 && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

#[derive(Deserialize)]
pub struct CreateEnvSpec {
    pub name: String,
    pub image: String,
    pub ports: Vec<PortMap>,
    pub preset: PackagePreset,
}

pub async fn list_docker_envs(docker: &Docker) -> Result<Vec<DockerContainer>, String> {
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec![LABEL.to_string()]);
    let containers = docker
        .list_containers(Some(ListContainersOptions { all: true, filters, ..Default::default() }))
        .await
        .map_err(|e| e.to_string())?;
    Ok(containers
        .into_iter()
        .filter_map(|c| {
            let env_name = c.labels.as_ref()?.get(LABEL)?.clone();
            Some(DockerContainer {
                env_name,
                image: c.image.unwrap_or_default(),
                running: c.state.as_deref() == Some("running"),
            })
        })
        .collect())
}

#[tauri::command]
pub async fn list_envs(state: tauri::State<'_, AppState>) -> Result<Vec<EnvView>, String> {
    let containers = list_docker_envs(&state.docker).await?;
    Ok(state.registry.lock().await.reconcile(&containers))
}

#[tauri::command]
pub async fn create_env(
    state: tauri::State<'_, AppState>,
    spec: CreateEnvSpec,
    on_progress: Channel<String>,
) -> Result<EnvView, String> {
    if !valid_name(&spec.name) {
        return Err("Name must be lowercase letters, digits, - or _ (max 31 chars).".into());
    }
    if state.registry.lock().await.get(&spec.name).is_some() {
        return Err(format!("Environment '{}' already exists.", spec.name));
    }

    // 1. Pull image, streaming progress lines to the UI.
    let mut pull = state.docker.create_image(
        Some(CreateImageOptions { from_image: spec.image.clone(), ..Default::default() }),
        None,
        None,
    );
    while let Some(item) = pull.next().await {
        let info = item.map_err(|e| format!("Image pull failed: {e}"))?;
        if let Some(s) = info.status {
            let _ = on_progress.send(match info.progress {
                Some(p) => format!("{s} {p}"),
                None => s,
            });
        }
    }

    let labels: HashMap<String, String> = HashMap::from([(LABEL.to_string(), spec.name.clone())]);

    // 2. Volume.
    state
        .docker
        .create_volume(CreateVolumeOptions { name: vol_name(&spec.name), labels: labels.clone(), ..Default::default() })
        .await
        .map_err(|e| format!("Volume creation failed: {e}"))?;

    // 3. Container with port bindings, workspace volume, sleep-infinity PID 1.
    let mut port_bindings = HashMap::new();
    let mut exposed = HashMap::new();
    for p in &spec.ports {
        let key = format!("{}/tcp", p.container);
        exposed.insert(key.clone(), HashMap::new());
        port_bindings.insert(
            key,
            Some(vec![PortBinding { host_ip: Some("127.0.0.1".into()), host_port: Some(p.host.to_string()) }]),
        );
    }
    let config = Config {
        image: Some(spec.image.clone()),
        cmd: Some(vec!["sleep".into(), "infinity".into()]),
        labels: Some(labels),
        working_dir: Some("/workspace".into()),
        exposed_ports: Some(exposed),
        host_config: Some(HostConfig {
            binds: Some(vec![format!("{}:/workspace", vol_name(&spec.name))]),
            port_bindings: Some(port_bindings),
            ..Default::default()
        }),
        ..Default::default()
    };
    let create_result = state
        .docker
        .create_container(Some(CreateContainerOptions { name: ctr_name(&spec.name), platform: None }), config)
        .await;
    if let Err(e) = create_result {
        // No orphan volume on failure.
        let _ = state.docker.remove_volume(&vol_name(&spec.name), None).await;
        return Err(format!("Container creation failed: {e}"));
    }

    if let Err(e) = state
        .docker
        .start_container(&ctr_name(&spec.name), None::<StartContainerOptions<String>>)
        .await
    {
        // No orphan container/volume on failure (label would make reconcile adopt a ghost env).
        let _ = state
            .docker
            .remove_container(&ctr_name(&spec.name), Some(RemoveContainerOptions { force: true, ..Default::default() }))
            .await;
        let _ = state.docker.remove_volume(&vol_name(&spec.name), None).await;
        return Err(format!("Container start failed: {e}"));
    }

    // 4. Optional package preset — non-fatal: the env is already created and
    // usable even if this install fails, so we only warn on the progress stream.
    if let Some(cmd) = install_command(spec.preset, pkg_manager_for_image(&spec.image)) {
        if let Err(e) = crate::files::exec_stream(&state, &ctr_name(&spec.name), cmd, &on_progress).await {
            let _ = on_progress.send(format!("[minive] package preset install failed (non-fatal): {e}"));
        }
    }

    let entry = EnvEntry { name: spec.name.clone(), image: spec.image.clone(), ports: spec.ports.clone() };
    state.registry.lock().await.upsert(entry);
    Ok(EnvView { name: spec.name, image: spec.image, ports: spec.ports, status: EnvStatus::Running })
}

#[tauri::command]
pub async fn start_env(state: tauri::State<'_, AppState>, name: String) -> Result<(), String> {
    state
        .docker
        .start_container(&ctr_name(&name), None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_env(state: tauri::State<'_, AppState>, name: String) -> Result<(), String> {
    state.docker.stop_container(&ctr_name(&name), None).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_env(state: tauri::State<'_, AppState>, name: String) -> Result<(), String> {
    // Force-remove container (ok if already gone), then volume, then registry entry.
    let _ = state
        .docker
        .remove_container(&ctr_name(&name), Some(RemoveContainerOptions { force: true, ..Default::default() }))
        .await;
    let _ = state.docker.remove_volume(&vol_name(&name), None).await;
    state.registry.lock().await.remove(&name);
    Ok(())
}
