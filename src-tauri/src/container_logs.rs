use bollard::container::LogsOptions;
use futures_util::StreamExt;
use tauri::ipc::Channel;

use crate::env_manager::ctr_name;
use crate::state::AppState;

#[tauri::command]
pub async fn stream_container_logs(
    state: tauri::State<'_, AppState>,
    name: String,
    on_output: Channel<String>,
) -> Result<(), String> {
    let my_gen = state.log_stream_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let mut stream = state.docker.logs(
        &ctr_name(&name),
        Some(LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            tail: "200".into(),
            ..Default::default()
        }),
    );
    while let Some(item) = stream.next().await {
        if state.log_stream_gen.load(std::sync::atomic::Ordering::SeqCst) != my_gen {
            break;
        }
        let msg = item.map_err(|e| e.to_string())?;
        // LogOutput has no Display/to_string in this bollard version (same pattern
        // as exec_stream in files.rs); decode the raw bytes lossily instead.
        let _ = on_output.send(String::from_utf8_lossy(&msg.into_bytes()).into_owned());
    }
    Ok(())
}
