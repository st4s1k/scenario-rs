use regex::Regex;
use std::{collections::HashMap, sync::mpsc::Sender};

/// Trait for safely sending events through a channel.
///
/// This trait provides a convenient wrapper around channel sending that handles
/// errors when the receiver has been dropped.
///
/// # Examples
///
/// ```
/// # use std::sync::mpsc;
/// # use scenario_rs::scenario::utils::SendEvent;
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
/// # use scenario_rs::scenario::utils::HasPlaceholders;
/// 
/// assert!(String::from("Hello, {name}!").has_placeholders());
/// assert!(!"Hello, world!".has_placeholders());
/// ```
pub(crate) trait HasPlaceholders
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
/// # use scenario_rs::scenario::utils::IsNotEmpty;
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
/// # use scenario_rs::scenario::utils::IsBlank;
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
/// # use scenario_rs::scenario::utils::HasText;
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
