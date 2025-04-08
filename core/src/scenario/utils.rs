use regex::Regex;
use std::sync::mpsc::Sender;

pub trait SendEvent<T> {
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
