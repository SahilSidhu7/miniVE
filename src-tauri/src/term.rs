use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecResults};
use futures_util::StreamExt;
use tauri::ipc::Channel;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::env_manager::ctr_name;
use crate::state::AppState;

/// Cap on the replay buffer kept per session. Trimming from the front can
/// split an ANSI escape sequence; xterm tolerates one garbled fragment on
/// replay, so that's an accepted trade for a hard memory bound.
const SCROLLBACK_MAX: usize = 256 * 1024;

pub struct Session {
    pub exec_id: String,
    pub input: Pin<Box<dyn AsyncWrite + Send>>,
    /// Every live viewer of this session (main-window tab, popped-out window).
    /// Dead channels are pruned when a send fails.
    subscribers: Vec<Channel<String>>,
    /// Recent output, replayed to late attachers so a popped-out window
    /// doesn't open onto a blank screen.
    scrollback: String,
}

fn push_scrollback(buf: &mut String, chunk: &str) {
    buf.push_str(chunk);
    if buf.len() > SCROLLBACK_MAX {
        let mut cut = buf.len() - SCROLLBACK_MAX;
        while !buf.is_char_boundary(cut) {
            cut += 1;
        }
        buf.drain(..cut);
    }
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
                // --norc so rc files can't clobber the git-bash-style PS1
                // passed via env (sh/ash honor the PS1 env var directly).
                cmd: Some(vec![
                    "sh".into(),
                    "-c".into(),
                    "command -v bash >/dev/null 2>&1 && exec bash --norc || exec sh".into(),
                ]),
                env: Some(vec![
                    "TERM=xterm-256color".into(),
                    // git-bash-style prompt: green user@host, magenta env tag,
                    // yellow cwd, then `$ ` on its own line.
                    format!(
                        "PS1=\\[\\e[32m\\]\\u@\\h \\[\\e[35m\\]{name} \\[\\e[33m\\]\\w\\[\\e[0m\\]\\n$ "
                    ),
                ]),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    match state.docker.start_exec(&exec.id, None).await.map_err(|e| e.to_string())? {
        StartExecResults::Attached { mut output, input } => {
            let id = state.next_session.fetch_add(1, Ordering::Relaxed);
            let session = Arc::new(Mutex::new(Session {
                exec_id: exec.id,
                input,
                subscribers: vec![on_data],
                scrollback: String::new(),
            }));
            let sessions = state.sessions.clone();
            sessions.lock().await.insert(id, session.clone());
            tauri::async_runtime::spawn(async move {
                while let Some(Ok(msg)) = output.next().await {
                    // ponytail: lossy utf8 on chunk boundaries; switch Channel to Vec<u8> if it ever garbles
                    let chunk = String::from_utf8_lossy(&msg.into_bytes()).into_owned();
                    let mut s = session.lock().await;
                    push_scrollback(&mut s.scrollback, &chunk);
                    s.subscribers.retain(|c| c.send(chunk.clone()).is_ok());
                }
                // Stream ended: drop our own entry so a stray write_terminal errors
                // cleanly instead of writing into a dead session forever.
                sessions.lock().await.remove(&id);
                for c in &session.lock().await.subscribers {
                    let _ = c.send("\r\n\x1b[90m[session ended]\x1b[0m\r\n".into());
                }
            });
            Ok(id)
        }
        StartExecResults::Detached => Err("exec unexpectedly detached".into()),
    }
}

/// Add another viewer to an existing session (pop-out window, re-attach).
/// Replays the scrollback first so the new view isn't blank. Returns the
/// channel id, which `detach_terminal` takes to remove this viewer later.
#[tauri::command]
pub async fn attach_terminal(
    state: tauri::State<'_, AppState>,
    id: u32,
    on_data: Channel<String>,
) -> Result<u32, String> {
    let session = {
        let sessions = state.sessions.lock().await;
        sessions.get(&id).ok_or("no such terminal session")?.clone()
    };
    let mut s = session.lock().await;
    if !s.scrollback.is_empty() {
        let _ = on_data.send(s.scrollback.clone());
    }
    let chan_id = on_data.id();
    s.subscribers.push(on_data);
    Ok(chan_id)
}

/// Remove one viewer without killing the session (the tab was popped out or
/// a window went away while other views remain).
#[tauri::command]
pub async fn detach_terminal(state: tauri::State<'_, AppState>, id: u32, channel_id: u32) -> Result<(), String> {
    let session = {
        let sessions = state.sessions.lock().await;
        match sessions.get(&id) {
            Some(s) => s.clone(),
            None => return Ok(()), // session already gone — nothing to detach
        }
    };
    session.lock().await.subscribers.retain(|c| c.id() != channel_id);
    Ok(())
}

#[tauri::command]
pub async fn write_terminal(state: tauri::State<'_, AppState>, id: u32, data: String) -> Result<(), String> {
    // Clone the Arc and drop the map lock before IO, so one stalled terminal can't block others.
    let session = {
        let sessions = state.sessions.lock().await;
        sessions.get(&id).ok_or("no such terminal session")?.clone()
    };
    let mut session = session.lock().await;
    session.input.write_all(data.as_bytes()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resize_terminal(state: tauri::State<'_, AppState>, id: u32, cols: u16, rows: u16) -> Result<(), String> {
    let session = {
        let sessions = state.sessions.lock().await;
        sessions.get(&id).ok_or("no such terminal session")?.clone()
    };
    let exec_id = session.lock().await.exec_id.clone();
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
