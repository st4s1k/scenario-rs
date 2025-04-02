use std::sync::{Mutex, MutexGuard};
use tauri::State;

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
