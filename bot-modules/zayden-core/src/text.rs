const FENCE: &str = "```";

#[must_use]
pub fn truncate(content: &str, limit: usize, ellipsis: &str) -> String {
    if content.chars().count() <= limit {
        return content.to_owned();
    }

    let budget = limit.saturating_sub(ellipsis.chars().count());

    let cut = content
        .char_indices()
        .nth(budget)
        .map_or(content.len(), |(index, _c)| index);
    let head = content.get(..cut).unwrap_or(content);

    let boundary = head
        .rfind("\n\n")
        .or_else(|| head.rfind('\n'))
        .filter(|index| *index * 2 > head.len())
        .unwrap_or(head.len());

    let mut out = head.get(..boundary).unwrap_or(head).trim_end().to_owned();

    close_fence(&mut out);
    out.push_str(ellipsis);
    out
}

pub fn close_fence(out: &mut String) {
    if out.matches(FENCE).count() % 2 == 1 {
        out.push('\n');
        out.push_str(FENCE);
    }
}
