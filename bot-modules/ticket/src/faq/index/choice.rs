use serenity::all::AutocompleteChoice;

const SENTINEL: &str = "faq://page/";
const CHOICE_LIMIT: usize = 100;
pub const MAX_CHOICES: usize = 25;

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

pub fn ask(query: &str) -> AutocompleteChoice<'static> {
    let label = if query.trim().is_empty() {
        "Ask a question".to_owned()
    } else {
        format!("Ask: {}", query.trim())
    };

    AutocompleteChoice::new(clamp(&label), clamp(query))
}

pub fn jump(label: &str, target: &Target) -> AutocompleteChoice<'static> {
    AutocompleteChoice::new(clamp(label), target.value())
}

fn clamp(text: &str) -> String {
    if text.chars().count() <= CHOICE_LIMIT {
        return text.to_owned();
    }

    text.chars().take(CHOICE_LIMIT - 1).collect::<String>() + "\u{2026}"
}
