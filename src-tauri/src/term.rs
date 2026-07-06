use std::pin::Pin;
use std::sync::atomic::Ordering;

use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecResults};
use futures_util::StreamExt;
use tauri::ipc::Channel;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::env_manager::ctr_name;
use crate::state::AppState;

pub struct Session {
    pub exec_id: String,
    pub input: Pin<Box<dyn AsyncWrite + Send>>,
}

#[tauri::command]
pub async fn open_terminal(
    state: tauri::State<'_, AppState>,
    name: String,
    on_data: Channel<String>,
) -> Result<u32, String> {
    let exec = state
        .docker
        .create_exec(
            &ctr_name(&name),
            CreateExecOptions::<String> {
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                tty: Some(true),
                working_dir: Some("/workspace".into()),
                cmd: Some(vec![
                    "sh".into(),
                    "-c".into(),
                    "command -v bash >/dev/null 2>&1 && exec bash || exec sh".into(),
                ]),
                env: Some(vec!["TERM=xterm-256color".into()]),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    match state.docker.start_exec(&exec.id, None).await.map_err(|e| e.to_string())? {
        StartExecResults::Attached { mut output, input } => {
            let id = state.next_session.fetch_add(1, Ordering::Relaxed);
            state.sessions.lock().await.insert(id, Session { exec_id: exec.id, input });
            tauri::async_runtime::spawn(async move {
                while let Some(Ok(msg)) = output.next().await {
                    // ponytail: lossy utf8 on chunk boundaries; switch Channel to Vec<u8> if it ever garbles
                    let _ = on_data.send(String::from_utf8_lossy(&msg.into_bytes()).into_owned());
                }
                let _ = on_data.send("\r\n\x1b[90m[session ended]\x1b[0m\r\n".into());
            });
            Ok(id)
        }
        StartExecResults::Detached => Err("exec unexpectedly detached".into()),
    }
}

#[tauri::command]
pub async fn write_terminal(state: tauri::State<'_, AppState>, id: u32, data: String) -> Result<(), String> {
    let mut sessions = state.sessions.lock().await;
    let session = sessions.get_mut(&id).ok_or("no such terminal session")?;
    session.input.write_all(data.as_bytes()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resize_terminal(state: tauri::State<'_, AppState>, id: u32, cols: u16, rows: u16) -> Result<(), String> {
    let exec_id = {
        let sessions = state.sessions.lock().await;
        sessions.get(&id).ok_or("no such terminal session")?.exec_id.clone()
    };
    state
        .docker
        .resize_exec(&exec_id, ResizeExecOptions { height: rows, width: cols })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_terminal(state: tauri::State<'_, AppState>, id: u32) -> Result<(), String> {
    // Dropping the input half closes stdin; the shell exits and the output task ends.
    state.sessions.lock().await.remove(&id);
    Ok(())
}
