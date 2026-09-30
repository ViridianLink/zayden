fn has_emoji(row: &str) -> bool {
    !row.trim().is_empty()
}

#[must_use]
pub fn with_emoji_row(head: &str, row: &str) -> String {
    if has_emoji(row) { format!("{head}\n#{row}") } else { head.to_owned() }
}

#[must_use]
pub fn emoji_section(title: &str, row: &str) -> Option<String> {
    has_emoji(row).then(|| format!("{title}\n#{row}"))
}
