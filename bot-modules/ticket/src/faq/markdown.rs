use url::Url;

use crate::wiki::WikiConfig;

pub const DESCRIPTION_LIMIT: usize = 4096;
pub const PROMPT_LIMIT: usize = 16_000;
const ELLIPSIS: &str = "\n\n_(truncated - read the full article via the title \
                        link)_";
const FENCE: &str = "```";

#[must_use]
pub fn truncate(content: &str, limit: usize) -> String {
    if content.chars().count() <= limit {
        return content.to_owned();
    }

    let budget = limit.saturating_sub(ELLIPSIS.chars().count());

    let cut = content
        .char_indices()
        .nth(budget)
        .map_or(content.len(), |(index, _c)| index);
    let head = content.get(..cut).unwrap_or(content);

    // Prefer the last blank line, then the last line break
    let boundary = head
        .rfind("\n\n")
        .or_else(|| head.rfind('\n'))
        .filter(|index| *index * 2 > head.len())
        .unwrap_or(head.len());

    let mut out = head.get(..boundary).unwrap_or(head).trim_end().to_owned();

    if out.matches(FENCE).count() % 2 == 1 {
        out.push('\n');
        out.push_str(FENCE);
    }

    out.push_str(ELLIPSIS);
    out
}

#[must_use]
pub fn take_first_image(content: &str, config: &WikiConfig) -> Option<Url> {
    let rest = content.split("![").nth(1)?;
    let target = rest.split_once("](")?.1.split(')').next()?.trim();
    let target = target.split_whitespace().next().unwrap_or(target);

    absolute(target, config)
}

#[must_use]
pub fn for_discord(content: &str, config: &WikiConfig) -> String {
    let stripped = strip_images(content);
    let stripped = strip_attribute_blocks(&stripped);
    absolutize_links(&stripped, config)
}

fn strip_images(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some((before, after)) = rest.split_once("![") {
        out.push_str(before);
        match after.split_once("](").and_then(|(_alt, tail)| tail.split_once(')')) {
            Some((_target, tail)) => rest = tail,
            None => {
                rest = after;
                break;
            },
        }
    }

    out.push_str(rest);
    out
}

fn strip_attribute_blocks(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some((before, after)) = rest.split_once("{class=") {
        out.push_str(before);
        match after.split_once('}') {
            Some((_attrs, tail)) => rest = tail,
            None => {
                rest = after;
                break;
            },
        }
    }

    out.push_str(rest);
    out
}

fn absolutize_links(content: &str, config: &WikiConfig) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some((before, after)) = rest.split_once("](/") {
        out.push_str(before);
        out.push_str("](");

        match after.split_once(')') {
            Some((target, tail)) => {
                match absolute(&format!("/{target}"), config) {
                    Some(url) => out.push_str(url.as_str()),
                    None => {
                        out.push('/');
                        out.push_str(target);
                    },
                }
                out.push(')');
                rest = tail;
            },
            None => {
                out.push('/');
                rest = after;
                break;
            },
        }
    }

    out.push_str(rest);
    out
}

fn absolute(target: &str, config: &WikiConfig) -> Option<Url> {
    if target.is_empty() {
        return None;
    }

    if let Ok(url) = Url::parse(target) {
        return matches!(url.scheme(), "http" | "https").then_some(url);
    }

    config.site_url(target).ok()
}
