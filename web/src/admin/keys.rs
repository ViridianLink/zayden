const EMOJI_CDN: &str = "https://cdn.discordapp.com/emojis/";
const MIN_KEY: usize = 2;
const MAX_KEY: usize = 32;
const SMALL_WORDS: [&str; 9] =
    ["a", "an", "and", "at", "for", "in", "of", "on", "the"];

#[must_use]
pub fn display_name(key: &str) -> String {
    key.split('_')
        .filter(|w| !w.is_empty())
        .enumerate()
        .map(|(i, word)| {
            if i > 0 && SMALL_WORDS.contains(&word) {
                return word.to_owned();
            }
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[must_use]
pub fn key_from_query(query: &str) -> String {
    let mut key = String::with_capacity(query.len());
    for c in query.trim().chars() {
        if c.is_ascii_alphanumeric() {
            key.push(c.to_ascii_lowercase());
        } else if (c.is_whitespace() || matches!(c, '_' | '-'))
            && !key.ends_with('_')
        {
            key.push('_');
        }
    }
    let key = key.trim_matches('_');
    key.chars().take(MAX_KEY).collect::<String>().trim_end_matches('_').to_owned()
}

#[must_use]
pub fn is_valid_key(key: &str) -> bool {
    (MIN_KEY..=MAX_KEY).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

#[must_use]
pub fn enum_key(label: &str) -> String {
    label.trim().to_lowercase().replace(' ', "_")
}

#[must_use]
pub fn emoji_url(id: &str) -> String {
    format!("{EMOJI_CDN}{id}.webp?size=64")
}
