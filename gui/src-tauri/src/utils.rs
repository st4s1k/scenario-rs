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

/// Trait for safely sending events through a channel.
///
/// This trait provides a convenient wrapper around channel sending that handles
/// errors when the receiver has been dropped.
///
/// # Examples
///
/// ```
/// # use std::sync::mpsc;
/// # use scenario_rs_core::utils::SendEvent;
///
/// let (tx, rx) = mpsc::channel();
/// tx.send_event("Scenario started");
/// assert_eq!(rx.recv().unwrap(), "Scenario started");
/// ```
pub trait SendEvent<T> {
    /// Sends an event through the channel, logging any errors if the channel is closed.
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
