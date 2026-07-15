//! Per-environment user scripts: stored in the registry, runnable from the
//! workspace Scripts panel, and (when `on_start` is set) executed
//! automatically on every container start.

use bollard::exec::{CreateExecOptions, StartExecResults};
use futures_util::StreamExt;
use tauri::ipc::Channel;

use crate::env_manager::ctr_name;
use crate::registry::ScriptEntry;
use crate::state::AppState;

#[tauri::command]
pub async fn list_scripts(state: tauri::State<'_, AppState>, name: String) -> Result<Vec<ScriptEntry>, String> {
    Ok(state
        .registry
        .lock()
        .await
        .get(&name)
        .map(|e| e.scripts.clone())
        .unwrap_or_default())
}

#[tauri::command]
pub async fn save_script(state: tauri::State<'_, AppState>, name: String, script: ScriptEntry) -> Result<(), String> {
    if script.name.trim().is_empty() {
        return Err("Script needs a name.".into());
    }
    state.registry.lock().await.upsert_script(&name, script)
}

#[tauri::command]
pub async fn delete_script(state: tauri::State<'_, AppState>, name: String, script_name: String) -> Result<(), String> {
    state.registry.lock().await.remove_script(&name, &script_name)
}

/// Run one stored script now, streaming combined output; returns exit code.
#[tauri::command]
pub async fn run_script(
    state: tauri::State<'_, AppState>,
    name: String,
    script_name: String,
    on_output: Channel<String>,
) -> Result<i64, String> {
    let content = state
        .registry
        .lock()
        .await
        .get(&name)
        .and_then(|e| e.scripts.iter().find(|s| s.name == script_name))
        .map(|s| s.content.clone())
        .ok_or("no such script")?;
    let cmd = vec!["sh".to_string(), "-c".to_string(), content];
    crate::files::exec_stream(&state, &ctr_name(&name), cmd, &on_output).await
}

/// Run a script with no live viewer (on-start automation): output is drained
/// and discarded, only the exit code matters.
pub async fn exec_silent(docker: &bollard::Docker, container: &str, content: &str) -> Result<(), String> {
    let exec = docker
        .create_exec(container, CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(vec!["sh".to_string(), "-c".to_string(), content.to_string()]),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;
    if let StartExecResults::Attached { mut output, .. } =
        docker.start_exec(&exec.id, None).await.map_err(|e| e.to_string())?
    {
        while let Some(_msg) = output.next().await {}
    }
    let inspect = docker.inspect_exec(&exec.id).await.map_err(|e| e.to_string())?;
    match inspect.exit_code {
        Some(0) | None => Ok(()),
        Some(code) => Err(format!("exit code {code}")),
    }
}
