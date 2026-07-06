// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod env_manager;
mod files;
mod registry;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let docker = bollard::Docker::connect_with_local_defaults()
                .expect("docker client construction cannot fail with defaults");
            let reg_path = app
                .path()
                .app_data_dir()
                .expect("app data dir")
                .join("registry.json");
            app.manage(AppState {
                docker,
                registry: tokio::sync::Mutex::new(registry::Registry::load(reg_path)),
                sessions: tokio::sync::Mutex::new(HashMap::new()),
                next_session: AtomicU32::new(1),
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
                    let mut events = docker.events(Some(bollard::system::EventsOptions::<String> {
                        filters,
                        ..Default::default()
                    }));
                    while let Some(Ok(_)) = events.next().await {
                        let _ = handle.emit("envs-changed", ());
                    }
                    // Stream ended or errored: daemon likely down. Poll until it's back.
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
            term::write_terminal,
            term::resize_terminal,
            term::close_terminal,
            files::upload_paths,
            files::clone_repo,
            files::list_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
