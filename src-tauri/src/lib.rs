// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod container_logs;
pub mod env_manager;
mod files;
mod images;
mod logging;
pub mod registry;
pub mod runtime_catalog;
mod settings;
mod state;
mod term;

use state::AppState;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use tauri::Manager;

#[tauri::command]
async fn docker_status(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(state.docker.ping().await.is_ok())
}

/// Silent startup check; asks before installing. Errors are logged, never fatal —
/// the app must stay usable offline or when GitHub is unreachable.
async fn check_for_updates(app: tauri::AppHandle) -> Result<(), tauri_plugin_updater::Error> {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
    use tauri_plugin_updater::UpdaterExt;

    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };
    let confirmed = app
        .dialog()
        .message(format!(
            "miniVE {} is available (you have {}). Install now?",
            update.version, update.current_version
        ))
        .title("Update available")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Install".into(),
            "Later".into(),
        ))
        .blocking_show();
    if confirmed {
        update.download_and_install(|_, _| {}, || {}).await?;
        app.restart();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let log_dir = app.path().app_data_dir().expect("app data dir").join("logs");
            let log_buffer = logging::init(&app.handle(), log_dir);

            let update_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = check_for_updates(update_handle).await {
                    tracing::warn!("update check failed: {e}");
                }
            });
            let docker = bollard::Docker::connect_with_local_defaults()
                .expect("docker client construction cannot fail with defaults");
            let reg_path = app
                .path()
                .app_data_dir()
                .expect("app data dir")
                .join("registry.json");
            let settings_path = app.path().app_data_dir().expect("app data dir").join("settings.json");
            let catalog_cache_path = app.path().app_data_dir().expect("app data dir").join("runtime_catalog_cache.json");
            app.manage(AppState {
                docker,
                registry: tokio::sync::Mutex::new(registry::Registry::load(reg_path)),
                settings: tokio::sync::Mutex::new(settings::Settings::load(settings_path)),
                catalog_cache_path,
                log_buffer,
                sessions: std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new())),
                next_session: AtomicU32::new(1),
                log_stream_gen: std::sync::atomic::AtomicU64::new(0),
            });

            let handle = app.handle().clone();
            let docker = bollard::Docker::connect_with_local_defaults().unwrap();
            tauri::async_runtime::spawn(async move {
                use futures_util::StreamExt;
                use tauri::Emitter;
                let mut was_up = true;
                loop {
                    let mut filters = std::collections::HashMap::new();
                    filters.insert("label".to_string(), vec![env_manager::LABEL.to_string()]);
                    filters.insert("type".to_string(), vec!["container".to_string()]);
                    // Only container lifecycle events matter for the env list; without this,
                    // exec_create/exec_start/exec_die from every terminal open/list_files also
                    // match and fire an envs-changed refresh (+ registry disk write) storm.
                    filters.insert(
                        "event".to_string(),
                        vec![
                            "start".to_string(),
                            "stop".to_string(),
                            "die".to_string(),
                            "destroy".to_string(),
                            "create".to_string(),
                        ],
                    );
                    let mut events = docker.events(Some(bollard::system::EventsOptions::<String> {
                        filters,
                        ..Default::default()
                    }));
                    while let Some(Ok(_)) = events.next().await {
                        let _ = handle.emit("envs-changed", ());
                    }
                    // Stream ended or errored. Could be a transient hiccup with the daemon
                    // still up, or the daemon actually going down - ping to tell which.
                    if docker.ping().await.is_ok() {
                        // transient stream hiccup; daemon fine - reconnect, but pause
                        // briefly so a persistently failing events endpoint can't
                        // turn this into a ping-speed busy loop.
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        continue;
                    }
                    if was_up {
                        let _ = handle.emit("docker-lost", ());
                        was_up = false;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    if docker.ping().await.is_ok() {
                        was_up = true;
                        let _ = handle.emit("docker-back", ());
                        let _ = handle.emit("envs-changed", ());
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            docker_status,
            env_manager::list_envs,
            env_manager::create_env,
            env_manager::start_env,
            env_manager::stop_env,
            env_manager::delete_env,
            term::open_terminal,
            term::attach_terminal,
            term::detach_terminal,
            term::write_terminal,
            term::resize_terminal,
            term::close_terminal,
            files::upload_paths,
            files::clone_repo,
            files::list_files,
            runtime_catalog::list_runtime_catalog,
            settings::pin_version,
            settings::unpin_version,
            settings::list_pinned_versions,
            images::list_cached_images,
            images::remove_cached_image,
            logging::get_backend_logs,
            container_logs::stream_container_logs,
            container_logs::stop_container_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
