use serenity::all::AutocompleteChoice;
use zayden_core::limits::autocomplete::{
    CHOICE_NAME as MAX_CHOICE_NAME,
    CHOICE_VALUE as MAX_CHOICE_VALUE,
};

const SENTINEL: &str = "faq://page/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub id: i32,
    pub anchor: Option<String>,
}

impl Target {
    #[must_use]
    pub fn value(&self) -> String {
        let value = self.anchor.as_ref().map_or_else(
            || format!("{SENTINEL}{}", self.id),
            |anchor| format!("{SENTINEL}{}#{anchor}", self.id),
        );

        clamp(&value)
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let rest = value.strip_prefix(SENTINEL)?;

        let (id, anchor) = match rest.split_once('#') {
            Some((id, anchor)) => (id, Some(anchor.to_owned())),
            None => (rest, None),
        };

        Some(Self { id: id.parse().ok()?, anchor })
    }
}

#[must_use]
pub fn ask(query: &str) -> Option<AutocompleteChoice<'static>> {
    let query = query.trim();

    if query.is_empty() {
        return None;
    }

    Some(AutocompleteChoice::new(clamp(&format!("Ask: {query}")), truncate(query)))
}

pub fn jump(label: &str, target: &Target) -> AutocompleteChoice<'static> {
    AutocompleteChoice::new(clamp(label), target.value())
}

fn clamp(text: &str) -> String {
    if text.chars().count() <= MAX_CHOICE_NAME {
        return text.to_owned();
    }

    text.chars().take(MAX_CHOICE_NAME - 1).collect::<String>() + "\u{2026}"
}

fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_CHOICE_VALUE {
        return text.to_owned();
    }

    let cut = text.chars().take(MAX_CHOICE_VALUE).collect::<String>();

    match cut.rsplit_once(char::is_whitespace) {
        Some((head, _tail)) if !head.trim().is_empty() => head.trim_end().to_owned(),
        _ => cut,
    }
}
