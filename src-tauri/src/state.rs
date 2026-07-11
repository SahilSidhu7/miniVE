use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::registry::Registry;
use crate::settings::Settings;
use crate::term::Session;

pub struct AppState {
    pub docker: bollard::Docker,
    pub registry: Mutex<Registry>,
    pub settings: Mutex<Settings>,
    pub catalog_cache_path: PathBuf,
    pub log_buffer: crate::logging::LogBuffer,
    // Per-session locks: outer map lock is held only for lookup, never across IO.
    // Wrapped in Arc so the terminal reader task can clone a handle and remove
    // its own entry when the output stream ends, without holding onto `state`.
    pub sessions: Arc<Mutex<HashMap<u32, Arc<Mutex<Session>>>>>,
    pub next_session: AtomicU32,
    pub log_stream_gen: std::sync::atomic::AtomicU64,
}
