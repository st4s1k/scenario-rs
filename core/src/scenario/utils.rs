use std::sync::mpsc::Sender;

use regex::Regex;

use super::events::ScenarioEvent;

pub(crate) trait HasPlaceholders
where
    Self: AsRef<str>,
{
    fn has_placeholders(&self) -> bool {
        let placeholder_regex =
            Regex::new(r"\{[^}]+\}").expect("`placeholder_regex` should be a valid regex");
        let value = self.as_ref();
        placeholder_regex.find(value).is_some()
    }
}

impl HasPlaceholders for String {}
impl HasPlaceholders for &str {}

pub (crate) trait SendEvent<T> {
    fn send_event(&self, event: T);
}

impl SendEvent<ScenarioEvent> for Sender<ScenarioEvent> {
    fn send_event(&self, event: ScenarioEvent) {
        if self.send(event.clone()).is_err() {
            eprintln!("Warning: Could not send event, channel may be closed");
            dbg!(event);
        }
    }
}
