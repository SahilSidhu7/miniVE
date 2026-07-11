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
        // ponytail: staleness is only checked between yielded lines, so a superseded
        // stream on a quiet container lingers until it next logs or the app exits —
        // bounded by container log frequency, not unbounded. Upgrade to
        // tokio::select! + a cancellation token if idle-stream teardown latency matters.
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

/// Bumps the generation counter without starting a new stream, so any
/// currently-running `stream_container_logs` task detects it's stale on its
/// next loop iteration and exits. Used when the logs panel closes or
/// switches away from the container tab, since nothing else would otherwise
/// stop an active `docker logs -f` stream.
#[tauri::command]
pub fn stop_container_logs(state: tauri::State<'_, AppState>) {
    state.log_stream_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}
