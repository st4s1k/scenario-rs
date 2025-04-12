use regex::Regex;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
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

#[cfg(test)]
mod tests {
    use super::{ArcMutex, HasPlaceholders, HasText, IsBlank, IsNotEmpty, Wrap};
    use std::{collections::HashMap, thread};

    #[test]
    fn test_arc_mutex_wrap_creates_shared_data() {
        // Given
        let initial_value = 42;

        // When
        let shared_value = ArcMutex::wrap(initial_value);

        // Then
        assert_eq!(*shared_value.lock().unwrap(), 42);
    }

    #[test]
    fn test_arc_mutex_wrap_allows_mutation() {
        // Given
        let counter = ArcMutex::wrap(0);

        // When
        {
            let mut value = counter.lock().unwrap();
            *value += 1;
        }

        // Then
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[test]
    fn test_arc_mutex_wrap_works_across_threads() {
        // Given
        let counter = ArcMutex::wrap(0);
        let counter_clone = counter.clone();

        // When
        let handle = thread::spawn(move || {
            let mut value = counter_clone.lock().unwrap();
            *value += 1;
        });

        // Wait for the thread to finish
        handle.join().unwrap();

        // Then
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[test]
    fn test_has_placeholders_with_placeholder() {
        // Given
        let text = "Hello, {name}!";

        // When & Then
        assert!(text.has_placeholders());
    }

    #[test]
    fn test_has_placeholders_without_placeholder() {
        // Given
        let text = "Hello, world!";

        // When & Then
        assert!(!text.has_placeholders());
    }

    #[test]
    fn test_has_placeholders_with_multiple_placeholders() {
        // Given
        let text = "Hello, {name}! Your {item} is ready.";

        // When & Then
        assert!(text.has_placeholders());
    }

    #[test]
    fn test_has_placeholders_with_nested_braces() {
        // Given
        let text = "This is not a {placeholder with {nested} braces}";

        // When & Then
        assert!(text.has_placeholders());
    }

    #[test]
    fn test_has_placeholders_with_string_type() {
        // Given
        let text = String::from("Hello, {name}!");

        // When & Then
        assert!(text.has_placeholders());
    }

    #[test]
    fn test_is_not_empty_with_non_empty_hashmap() {
        // Given
        let mut map = HashMap::new();
        map.insert("key", "value");

        // When & Then
        assert!(map.is_not_empty());
    }

    #[test]
    fn test_is_not_empty_with_empty_hashmap() {
        // Given
        let map: HashMap<&str, &str> = HashMap::new();

        // When & Then
        assert!(!map.is_not_empty());
    }

    #[test]
    fn test_is_not_empty_with_non_empty_vector() {
        // Given
        let vec = vec![1, 2, 3];

        // When & Then
        assert!(vec.is_not_empty());
    }

    #[test]
    fn test_is_not_empty_with_empty_vector() {
        // Given
        let vec: Vec<i32> = Vec::new();

        // When & Then
        assert!(!vec.is_not_empty());
    }

    #[test]
    fn test_is_blank_with_empty_string() {
        // Given
        let text = "";

        // When & Then
        assert!(text.is_blank());
    }

    #[test]
    fn test_is_blank_with_whitespace_string() {
        // Given
        let text = "   \t\n  ";

        // When & Then
        assert!(text.is_blank());
    }

    #[test]
    fn test_is_blank_with_non_blank_string() {
        // Given
        let text = "Hello";

        // When & Then
        assert!(!text.is_blank());
    }

    #[test]
    fn test_is_blank_with_string_containing_whitespace() {
        // Given
        let text = "  Hello  ";

        // When & Then
        assert!(!text.is_blank());
    }

    #[test]
    fn test_has_text_with_non_empty_string() {
        // Given
        let text = "Hello";

        // When & Then
        assert!(text.has_text());
    }

    #[test]
    fn test_has_text_with_empty_string() {
        // Given
        let text = "";

        // When & Then
        assert!(!text.has_text());
    }

    #[test]
    fn test_has_text_with_whitespace_string() {
        // Given
        let text = "   \t\n  ";

        // When & Then
        assert!(!text.has_text());
    }
}
