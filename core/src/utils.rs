use regex::Regex;
use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc, Mutex},
};

/// A convenience type alias for an `Arc<Mutex<T>>`.
///
/// This type combines atomic reference counting (`Arc`) with mutual exclusion (`Mutex`),
/// allowing for shared mutable state across threads. It's particularly useful for:
///
/// - Safely sharing mutable data between tasks or components
/// - Implementing thread-safe caching mechanisms
/// - Managing concurrent access to resources like connections
///
/// # Examples
///
/// ```
/// use scenario_rs_core::utils::{ArcMutex, Wrap};
///
/// // Create a thread-safe counter
/// let counter = ArcMutex::wrap(0);
///
/// // Clone the reference to share between threads
/// let counter_clone = counter.clone();
///
/// // Use in a separate thread
/// std::thread::spawn(move || {
///     let mut value = counter_clone.lock().unwrap();
///     *value += 1;
/// });
///
/// // Access the value in the main thread
/// {
///     let mut value = counter.lock().unwrap();
///     *value += 1;
/// }
/// ```
///
/// # Thread Safety
///
/// `ArcMutex<T>` is both `Send` and `Sync` when `T` is `Send`, making it safe
/// to share between threads.
pub type ArcMutex<T> = Arc<Mutex<T>>;

/// A trait for wrapping values in a container.
///
/// This trait provides a consistent way to wrap values in various container types.
/// It's primarily used with `ArcMutex<T>` to simplify the creation of thread-safe
/// mutable state.
///
/// # Examples
///
/// ```
/// use scenario_rs_core::utils::{ArcMutex, Wrap};
///
/// // Create a shared counter
/// let counter = ArcMutex::wrap(0);
///
/// // Create a shared string
/// let message = ArcMutex::wrap(String::from("Hello"));
/// ```
pub trait Wrap<T> {
    /// Wraps a value of type `T` in the implementing container type.
    ///
    /// # Arguments
    ///
    /// * `data` - The value to wrap
    ///
    /// # Returns
    ///
    /// The wrapped value
    fn wrap(data: T) -> Self;
}

impl<T> Wrap<T> for ArcMutex<T> {
    /// Wraps a value in an `ArcMutex<T>`.
    ///
    /// This is a convenience method that combines `Arc::new` and `Mutex::new`
    /// into a single operation.
    ///
    /// # Arguments
    ///
    /// * `data` - The value to wrap in an `Arc<Mutex<T>>`
    ///
    /// # Returns
    ///
    /// An `ArcMutex<T>` containing the provided value
    fn wrap(data: T) -> Self {
        Arc::new(Mutex::new(data))
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
            eprintln!(
                "Failed to send event {:?} (channel closed): {:?}",
                event, err
            );
        }
    }
}

/// Checks if a string contains placeholder variables in the format {variable_name}.
///
/// Placeholders are identified by curly braces surrounding the variable name.
///
/// # Examples
///
/// ```
/// # use scenario_rs_core::utils::HasPlaceholders;
///
/// assert!(String::from("Hello, {name}!").has_placeholders());
/// assert!(!"Hello, world!".has_placeholders());
/// ```
pub trait HasPlaceholders
where
    Self: AsRef<str>,
{
    fn has_placeholders(&self) -> bool {
        let placeholder_regex =
            Regex::new(r"\{[^}]+\}").expect("placeholder_regex should be a valid regex");

        placeholder_regex.find(self.as_ref()).is_some()
    }
}

impl HasPlaceholders for String {}
impl HasPlaceholders for &str {}

/// Utility trait to check if a collection is not empty.
///
/// This provides a more readable alternative to `!collection.is_empty()`.
///
/// # Examples
///
/// ```
/// # use scenario_rs_core::utils::IsNotEmpty;
/// # use std::collections::HashMap;
///
/// let mut map = HashMap::new();
/// map.insert("key", "value");
/// assert!(map.is_not_empty());
///
/// let vec = vec![1, 2, 3];
/// assert!(vec.is_not_empty());
/// ```
pub trait IsNotEmpty {
    fn is_not_empty(&self) -> bool;
}

impl<K, V> IsNotEmpty for HashMap<K, V> {
    fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }
}

impl<T> IsNotEmpty for Vec<T> {
    fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }
}

/// Utility trait to check if a string is blank (empty or only whitespace).
///
/// # Examples
///
/// ```
/// # use scenario_rs_core::utils::IsBlank;
///
/// assert!("".is_blank());
/// assert!("   ".is_blank());
/// assert!(!("Hello".is_blank()));
/// ```
pub trait IsBlank {
    fn is_blank(&self) -> bool;
}

impl<T: AsRef<str>> IsBlank for T {
    fn is_blank(&self) -> bool {
        self.as_ref().trim().is_empty()
    }
}

/// Utility trait to check if a string contains any non-whitespace text.
///
/// This is the logical complement to the `IsBlank` trait.
///
/// # Examples
///
/// ```
/// # use scenario_rs_core::utils::HasText;
///
/// assert!("Hello".has_text());
/// assert!(!("").has_text());
/// assert!(!("   ".has_text()));
/// ```
pub trait HasText {
    fn has_text(&self) -> bool;
}

impl<T: IsBlank> HasText for T {
    fn has_text(&self) -> bool {
        !self.is_blank()
    }
}
