use regex::Regex;
use std::{collections::HashMap, sync::mpsc::Sender};

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

pub trait IsBlank {
    fn is_blank(&self) -> bool;
}

impl<T: AsRef<str>> IsBlank for T {
    fn is_blank(&self) -> bool {
        self.as_ref().trim().is_empty()
    }
}

pub trait HasText {
    fn has_text(&self) -> bool;
}

impl<T: IsBlank> HasText for T {
    fn has_text(&self) -> bool {
        !self.is_blank()
    }
}
