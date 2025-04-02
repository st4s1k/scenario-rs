use regex::Regex;
use std::sync::mpsc::Sender;

pub trait SendEvent<T> {
    fn send_event(&self, event: T);
}

impl<T: Clone + std::fmt::Debug> SendEvent<T> for Sender<T> {
    fn send_event(&self, event: T) {
        if let Err(_) = self.send(event.clone()) {
            eprintln!(
                "Warning: Failed to send {} event (channel closed)",
                std::any::type_name::<T>()
            );
            #[cfg(debug_assertions)]
            eprintln!("  Event details: {:?}", event);
        }
    }
}

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
