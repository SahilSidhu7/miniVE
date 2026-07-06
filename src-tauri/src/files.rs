use bollard::container::UploadToContainerOptions;
use bollard::exec::{CreateExecOptions, StartExecResults};
use futures_util::StreamExt;
use serde::Serialize;
use tauri::ipc::Channel;

use crate::env_manager::ctr_name;
use crate::state::AppState;

#[derive(Serialize)]
pub struct FileNode {
    pub name: String,
    pub is_dir: bool,
}

#[tauri::command]
pub async fn upload_paths(state: tauri::State<'_, AppState>, name: String, paths: Vec<String>) -> Result<(), String> {
    let tar_bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let mut builder = tar::Builder::new(Vec::new());
        for p in &paths {
            let path = std::path::PathBuf::from(p);
            let file_name = path
                .file_name()
                .ok_or_else(|| format!("bad path: {p}"))?
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                builder.append_dir_all(&file_name, &path).map_err(|e| e.to_string())?;
            } else {
                builder.append_path_with_name(&path, &file_name).map_err(|e| e.to_string())?;
            }
        }
        builder.into_inner().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    state
        .docker
        .upload_to_container(
            &ctr_name(&name),
            Some(UploadToContainerOptions { path: "/workspace".to_string(), ..Default::default() }),
            tar_bytes.into(),
        )
        .await
        .map_err(|e| format!("Upload failed: {e}"))
}

/// Streams combined output lines to `on_output`; returns exit code.
async fn exec_stream(
    state: &AppState,
    container: &str,
    cmd: Vec<String>,
    on_output: &Channel<String>,
) -> Result<i64, String> {
    let exec = state
        .docker
        .create_exec(container, CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(cmd),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;
    if let StartExecResults::Attached { mut output, .. } =
        state.docker.start_exec(&exec.id, None).await.map_err(|e| e.to_string())?
    {
        while let Some(Ok(msg)) = output.next().await {
            let _ = on_output.send(String::from_utf8_lossy(&msg.into_bytes()).into_owned());
        }
    }
    let inspect = state.docker.inspect_exec(&exec.id).await.map_err(|e| e.to_string())?;
    Ok(inspect.exit_code.unwrap_or(-1))
}

#[tauri::command]
pub async fn clone_repo(
    state: tauri::State<'_, AppState>,
    name: String,
    url: String,
    on_output: Channel<String>,
) -> Result<i64, String> {
    // URL passed as $0 positional arg — never interpolated into the script (shell-injection safe).
    let script = r#"command -v git >/dev/null 2>&1 || { echo '[minive] installing git...'; apt-get update -qq && apt-get install -y -qq git || apk add --no-cache git; }; cd /workspace && git clone --progress "$0" 2>&1"#;
    exec_stream(&state, &ctr_name(&name), vec!["sh".into(), "-c".into(), script.into(), url], &on_output).await
}

#[tauri::command]
pub async fn list_files(state: tauri::State<'_, AppState>, name: String, path: String) -> Result<Vec<FileNode>, String> {
    if path.contains("..") {
        return Err("invalid path".into());
    }
    let abs = format!("/workspace/{}", path.trim_start_matches('/'));
    // Collect output rather than stream.
    let exec = state
        .docker
        .create_exec(&ctr_name(&name), CreateExecOptions::<String> {
            attach_stdout: Some(true),
            cmd: Some(vec!["sh".into(), "-c".into(), r#"ls -1Ap "$0" 2>/dev/null"#.into(), abs]),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;
    let mut out = String::new();
    if let StartExecResults::Attached { mut output, .. } =
        state.docker.start_exec(&exec.id, None).await.map_err(|e| e.to_string())?
    {
        while let Some(Ok(msg)) = output.next().await {
            out.push_str(&String::from_utf8_lossy(&msg.into_bytes()));
        }
    }
    Ok(out
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| FileNode { name: l.trim_end_matches('/').to_string(), is_dir: l.ends_with('/') })
        .collect())
}
