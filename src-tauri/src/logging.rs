use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub const MAX_LINES: usize = 2000;

pub type LogBuffer = Arc<Mutex<VecDeque<String>>>;

/// Pushes a line into the buffer, evicting the oldest line first if at capacity.
/// Extracted as a pure function (buffer passed in) so eviction behavior is
/// unit-testable without spinning up a real tracing subscriber.
pub fn push_and_format(buffer: &LogBuffer, max_lines: usize, line: String) {
    let mut buf = buffer.lock().unwrap();
    if buf.len() >= max_lines {
        buf.pop_front();
    }
    buf.push_back(line);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushes_lines_in_order() {
        let buf: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
        push_and_format(&buf, 10, "a".into());
        push_and_format(&buf, 10, "b".into());
        assert_eq!(buf.lock().unwrap().iter().cloned().collect::<Vec<_>>(), vec!["a", "b"]);
    }

    #[test]
    fn evicts_oldest_when_at_capacity() {
        let buf: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
        push_and_format(&buf, 2, "a".into());
        push_and_format(&buf, 2, "b".into());
        push_and_format(&buf, 2, "c".into());
        assert_eq!(buf.lock().unwrap().iter().cloned().collect::<Vec<_>>(), vec!["b", "c"]);
    }
}

use tauri::{AppHandle, Emitter};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;

struct EmitLayer {
    buffer: LogBuffer,
    app: AppHandle,
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }
}

impl<S> Layer<S> for EmitLayer
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let line = format!("[{}] {}", event.metadata().level(), visitor.message);
        push_and_format(&self.buffer, MAX_LINES, line.clone());
        let _ = self.app.emit("backend-log", line);
    }
}

/// Must be called from inside Tauri's `.setup()` closure — needs a live `AppHandle`
/// to emit live log events, which isn't available before the app is built.
pub fn init(app: &AppHandle, log_dir: std::path::PathBuf) -> LogBuffer {
    let buffer: LogBuffer = Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LINES)));
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::daily(&log_dir, "minive.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    // Leaked deliberately: the guard must live for the process lifetime to keep
    // flushing the non-blocking file writer; there's no natural owner to hold it.
    std::mem::forget(guard);
    let file_layer = tracing_subscriber::fmt::layer().with_writer(non_blocking).with_ansi(false);
    let emit_layer = EmitLayer { buffer: buffer.clone(), app: app.clone() };
    tracing_subscriber::registry().with(file_layer).with(emit_layer).init();
    buffer
}

#[tauri::command]
pub fn get_backend_logs(state: tauri::State<'_, crate::state::AppState>) -> Vec<String> {
    state.log_buffer.lock().unwrap().iter().cloned().collect()
}
