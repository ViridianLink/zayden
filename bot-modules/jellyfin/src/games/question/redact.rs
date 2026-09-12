const PLACEHOLDER: &str = "[REDACTED]";

const COMMON: &[&str] = &[
    "a", "an", "and", "after", "all", "as", "at", "but", "by", "for", "from", "he",
    "her", "his", "how", "i", "in", "is", "it", "its", "of", "on", "one", "or",
    "she", "the", "their", "them", "they", "this", "to", "two", "up", "when",
    "while", "who", "with", "years", "year", "world", "life", "man", "woman", "boy",
    "girl", "family", "town", "city", "war", "day", "night",
];

#[must_use]
pub fn redact(overview: &str, title: &str) -> String {
    let title_words: Vec<String> = title
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|w| w.len() > 2)
        .collect();

    let mut out = String::with_capacity(overview.len());
    let mut sentence_start = true;

    for word in overview.split_inclusive(char::is_whitespace) {
        let trimmed = word.trim();
        let core = trimmed.trim_matches(|c: char| !c.is_alphanumeric());
        let lowered = core.to_lowercase();

        let is_name = !core.is_empty()
            && core.chars().next().is_some_and(char::is_uppercase)
            && !COMMON.contains(&lowered.as_str())
            && (!sentence_start || title_words.contains(&lowered));

        // The title itself is always redacted, wherever it appears.
        let redact_this = is_name || title_words.contains(&lowered);

        if redact_this {
            out.push_str(PLACEHOLDER);
            if word.ends_with(char::is_whitespace) {
                out.push(' ');
            }
        } else {
            out.push_str(word);
        }

        sentence_start = trimmed.ends_with('.')
            || trimmed.ends_with('!')
            || trimmed.ends_with('?');
    }

    out
}
