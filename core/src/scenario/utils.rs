use regex::Regex;

pub(crate) trait HasPlaceholders
where
    Self: AsRef<str>,
{
    fn has_placeholders(&self) -> bool {
        let placeholder_regex = Regex::new(r"\{\w+}")
            .expect("`placeholder_regex` should be a valid regex");
        let value = self.as_ref();
        placeholder_regex.find(value).is_some()
    }
}

impl HasPlaceholders for String {}
impl HasPlaceholders for &str {}
