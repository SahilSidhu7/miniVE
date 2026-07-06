use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::registry::Registry;
use crate::term::Session;

pub struct AppState {
    pub docker: bollard::Docker,
    pub registry: Mutex<Registry>,
    // Per-session locks: outer map lock is held only for lookup, never across IO.
    pub sessions: Mutex<HashMap<u32, Arc<Mutex<Session>>>>,
    pub next_session: AtomicU32,
}
