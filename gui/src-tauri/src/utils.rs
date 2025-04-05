use crate::app::ScenarioAppState;
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Emitter, Manager, State};
use chrono::Local;

pub trait SafeLock<T: Send> {
    fn safe_lock(&self) -> MutexGuard<'_, T>;
}

impl<'a, T: Send> SafeLock<T> for State<'a, Mutex<T>> {
    fn safe_lock(&self) -> MutexGuard<'_, T> {
        match self.lock() {
            Ok(guard) => guard,
            Err(poison_error) => {
                eprintln!(
                    "WARNING: Recovered from mutex poison error: {:?}",
                    poison_error
                );
                poison_error.into_inner()
            }
        }
    }
}

pub trait LogMessage {
    fn log_message(&self, message: impl AsRef<str>);
}

impl LogMessage for AppHandle {
    fn log_message(&self, message: impl AsRef<str>) {
        let state = self.state::<Mutex<ScenarioAppState>>();
        let mut state = state.safe_lock();
        let timestamp = Local::now().format("%H:%M:%S.%3f").to_string();
        state.output_log.push_str(&format!("[{}] {}\n", timestamp, message.as_ref()));
        let _ = self.emit("log-update", ());
    }
}
