// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![docker_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
