use std::sync::{mpsc::Sender, Mutex, MutexGuard};
use tauri::State;
use tracing::error;

#[cfg(not(tarpaulin_include))]
pub trait SafeLock<T: Send> {
    fn safe_lock(&self) -> MutexGuard<'_, T>;
}

#[cfg(not(tarpaulin_include))]
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

fn log_send_error(msg: &str) {
    error!("{}", msg);
}

impl<T: Clone + std::fmt::Debug> SendEvent<T> for Sender<T> {
    fn send_event(&self, event: T) {
        if let Err(err) = self.send(event.clone()) {
            let msg = format!(
                "Failed to send event {:?} (channel closed): {:?}",
                event, err
            );
            log_send_error(&msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::TRACE)
            .try_init();
    }

    #[test]
    fn test_send_event_delivers_message_successfully() {
        // Given
        let (sender, receiver) = mpsc::channel::<String>();

        // When
        sender.send_event("hello".to_string());

        // Then
        assert_eq!(receiver.try_recv().unwrap(), "hello");
    }

    #[test]
    fn test_send_event_delivers_multiple_messages() {
        // Given
        let (sender, receiver) = mpsc::channel::<i32>();

        // When
        sender.send_event(1);
        sender.send_event(2);
        sender.send_event(3);

        // Then
        assert_eq!(receiver.try_recv().unwrap(), 1);
        assert_eq!(receiver.try_recv().unwrap(), 2);
        assert_eq!(receiver.try_recv().unwrap(), 3);
    }

    #[test]
    fn test_send_event_does_not_panic_when_receiver_dropped() {
        init_tracing();

        // Given
        let (sender, receiver) = mpsc::channel::<String>();
        drop(receiver);

        // When & Then
        sender.send_event("orphaned".to_string());
    }
}
