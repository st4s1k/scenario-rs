use regex::Regex;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Convenience alias for `Arc<Mutex<T>>`.
pub type ArcMutex<T> = Arc<Mutex<T>>;

/// Wraps a value into a container type.
pub trait Wrap<T> {
    fn wrap(data: T) -> Self;
}

impl<T> Wrap<T> for ArcMutex<T> {
    fn wrap(data: T) -> Self {
        Arc::new(Mutex::new(data))
    }
}

/// Checks if a string contains `{variable_name}` placeholders.
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

/// More readable alternative to `!collection.is_empty()`.
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

/// Checks if a string is empty or only whitespace.
pub trait IsBlank {
    fn is_blank(&self) -> bool;
}

impl<T: AsRef<str>> IsBlank for T {
    fn is_blank(&self) -> bool {
        self.as_ref().trim().is_empty()
    }
}

/// Checks if a string contains any non-whitespace text. Complement of `IsBlank`.
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
    use crate::utils::{ArcMutex, HasPlaceholders, HasText, IsBlank, IsNotEmpty, Wrap};
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
