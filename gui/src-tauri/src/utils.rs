use std::sync::{mpsc::Sender, Mutex, MutexGuard};
use tauri::State;
use tracing::error;

pub trait SafeLock<T: Send> {
    fn safe_lock(&self) -> MutexGuard<'_, T>;
}

impl<'a, T: Send> SafeLock<T> for State<'a, Mutex<T>> {
    fn safe_lock(&self) -> MutexGuard<'_, T> {
        match self.lock() {
            Ok(guard) => guard,
            Err(poison_error) => {
                error!(
                    "WARNING: Recovered from mutex poison error: {:?}",
                    poison_error
                );
                poison_error.into_inner()
            }
        }
    }
}

/// Trait for safely sending events through a channel, logging errors if the receiver is dropped.
pub trait SendEvent<T> {
    fn send_event(&self, event: T);
}

impl<T: Clone + std::fmt::Debug> SendEvent<T> for Sender<T> {
    fn send_event(&self, event: T) {
        if let Err(err) = self.send(event.clone()) {
            error!(
                "Failed to send event {:?} (channel closed): {:?}",
                event, err
            );
        }
    }
}
