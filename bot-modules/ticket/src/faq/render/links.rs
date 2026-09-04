use url::Url;

use crate::wiki::WikiConfig;

const CHROME: &[&str] = &["icon", "favicon", "logo", "badge", "avatar", "emoji"];

pub(crate) fn strip_images(content: &str) -> String {
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

pub(crate) fn strip_attribute_blocks(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(index) = rest.find('{') {
        let (before, tail) = rest.split_at_checked(index).unwrap_or((rest, ""));
        out.push_str(before);

        match attribute_block(tail) {
            Some(after) => rest = after,
            None => {
                out.push('{');
                rest = tail.get(1..).unwrap_or_default();
            },
        }
    }

    out.push_str(rest);
    out
}

fn attribute_block(input: &str) -> Option<&str> {
    let body = input.get(1..)?;
    let end = body.find('}')?;
    let inner = body.get(..end)?;

    let attributes = inner.starts_with(['.', '#'])
        || inner.split_once('=').is_some_and(|(key, _value)| {
            !key.is_empty()
                && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });

    attributes.then(|| body.get(end + 1..)).flatten()
}

pub(crate) fn absolutize(content: &str, config: &WikiConfig) -> String {
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

#[must_use]
pub fn thumbnail(content: &str, config: &WikiConfig) -> Option<Url> {
    content
        .lines()
        .filter(|line| !line.trim_start().starts_with('|'))
        .flat_map(targets)
        .find(|(target, attributes)| !is_chrome(target, attributes))
        .and_then(|(target, _attributes)| absolute(target, config))
}

fn targets(line: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut rest = line;

    while let Some((_before, after)) = rest.split_once("![") {
        let Some((_alt, tail)) = after.split_once("](") else {
            break;
        };

        let Some((target, tail)) = tail.split_once(')') else {
            break;
        };

        rest = tail;

        let attributes = tail
            .strip_prefix('{')
            .and_then(|block| block.split_once('}'))
            .map_or("", |(attributes, _tail)| attributes);

        let target = target.trim();
        let target = target.split_whitespace().next().unwrap_or(target);

        if !target.is_empty() {
            found.push((target, attributes));
        }
    }

    found
}

fn is_chrome(target: &str, attributes: &str) -> bool {
    let name = target.rsplit('/').next().unwrap_or(target).to_ascii_lowercase();
    let attributes = attributes.to_ascii_lowercase();

    CHROME.iter().any(|marker| name.contains(marker) || attributes.contains(marker))
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
