use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use tokio::sync::Mutex;

use crate::registry::Registry;
use crate::term::Session;

pub struct AppState {
    pub docker: bollard::Docker,
    pub registry: Mutex<Registry>,
    pub sessions: Mutex<HashMap<u32, Session>>,
    pub next_session: AtomicU32,
}
