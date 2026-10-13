// CommonMark HTML block start condition 6: https://spec.commonmark.org/0.31.2/#html-blocks
const BLOCK_TAGS: &[&str] = &[
    "aside",
    "blockquote",
    "center",
    "details",
    "div",
    "dl",
    "figure",
    "nav",
    "ol",
    "p",
    "section",
    "table",
    "ul",
];
const BREAKS: &[&str] = &[
    "aside",
    "blockquote",
    "br",
    "center",
    "dd",
    "details",
    "div",
    "dt",
    "figure",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "li",
    "nav",
    "p",
    "section",
    "summary",
    "tr",
];

pub(crate) fn flatten(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut block = String::new();

    for line in content.split_inclusive('\n') {
        if block.is_empty() && !opens_block(line) {
            out.push_str(line);
        } else if line.trim().is_empty() {
            push_block(&mut out, &std::mem::take(&mut block));
            out.push_str(line);
        } else {
            block.push_str(line);
        }
    }

    push_block(&mut out, &block);
    out
}

fn opens_block(line: &str) -> bool {
    let trimmed = line.trim_start_matches(' ');

    if line.len() - trimmed.len() > 3 {
        return false;
    }

    Tag::parse(trimmed).is_some_and(|tag| BLOCK_TAGS.contains(&tag.name.as_str()))
}

fn push_block(out: &mut String, block: &str) {
    for line in convert(block).lines().map(str::trim).filter(|line| !line.is_empty())
    {
        out.push_str(line);
        out.push('\n');
    }
}

fn convert(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut links: Vec<Option<String>> = Vec::new();
    let mut rest = html;

    while let Some(index) = rest.find('<') {
        let (before, tail) = rest.split_at_checked(index).unwrap_or((rest, ""));
        out.push_str(before);

        let Some(tag) = Tag::parse(tail) else {
            out.push('<');
            rest = tail.get(1..).unwrap_or_default();
            continue;
        };

        match (tag.name.as_str(), tag.closing) {
            ("b" | "strong", _) => out.push_str("**"),
            ("code", _) => out.push('`'),
            ("a", false) => {
                let href = tag.href();
                if href.is_some() {
                    out.push('[');
                }
                links.push(href);
            },
            ("a", true) => {
                if let Some(Some(href)) = links.pop() {
                    out.push_str("](");
                    out.push_str(&href);
                    out.push(')');
                }
            },
            ("span", true) if starts_sibling(tag.rest) => out.push_str(": "),
            (name, _) if BREAKS.contains(&name) => out.push('\n'),
            _ => out.push_str(tag.raw),
        }

        rest = tag.rest;
    }

    out.push_str(rest);
    out
}

fn starts_sibling(rest: &str) -> bool {
    rest.starts_with('<') && !rest.starts_with("</")
}

struct Tag<'a> {
    name: String,
    closing: bool,
    attrs: &'a str,
    raw: &'a str,
    rest: &'a str,
}

impl<'a> Tag<'a> {
    fn parse(input: &'a str) -> Option<Self> {
        let after = input.strip_prefix('<')?;
        let (closing, after) =
            after.strip_prefix('/').map_or((false, after), |tail| (true, tail));

        if !after.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return None;
        }

        let end = after
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .unwrap_or(after.len());
        let name = after.get(..end)?.to_ascii_lowercase();
        let tail = after.get(end..)?;

        if !tail.starts_with(['>', '/', ' ', '\t', '\n', '\r']) {
            return None;
        }

        let close = close_bracket(tail)?;
        let attrs = tail.get(..close)?;
        let rest = tail.get(close + 1..)?;
        let raw = input.get(..input.len() - rest.len())?;

        Some(Self { name, closing, attrs, raw, rest })
    }

    fn href(&self) -> Option<String> {
        let value = self.attrs.split_once("href=")?.1;
        let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''))?;
        let (href, _rest) = value.get(1..)?.split_once(quote)?;

        (!href.is_empty()).then(|| href.to_owned())
    }
}

fn close_bracket(tail: &str) -> Option<usize> {
    let mut quote = None;

    for (index, c) in tail.char_indices() {
        match (quote, c) {
            (Some(open), c) if c == open => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            (None, '>') => return Some(index),
            (Some(_) | None, _) => {},
        }
    }

    None
}
