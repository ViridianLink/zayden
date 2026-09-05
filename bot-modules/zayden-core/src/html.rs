const DISCARD_CONTENT: &[&str] = &["script", "style", "svg", "iframe"];

const NAMED: &[(&str, &str)] = &[
    ("amp", "&"),
    ("lt", "<"),
    ("gt", ">"),
    ("quot", "\""),
    ("apos", "'"),
    ("nbsp", " "),
    ("ensp", " "),
    ("emsp", " "),
    ("thinsp", " "),
    ("mdash", "-"),
    ("ndash", "-"),
    ("hellip", "..."),
    ("lsquo", "'"),
    ("rsquo", "'"),
    ("ldquo", "\""),
    ("rdquo", "\""),
    ("times", "x"),
    ("middot", "\u{b7}"),
    ("bull", "\u{2022}"),
    ("copy", "\u{a9}"),
    ("reg", "\u{ae}"),
    ("deg", "\u{b0}"),
];

#[must_use]
pub fn strip(content: &str) -> String {
    decode_entities(&strip_tags(content))
}

fn strip_tags(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(index) = rest.find('<') {
        let (before, tail) = rest.split_at_checked(index).unwrap_or((rest, ""));

        let Some(tag) = Tag::parse(tail) else {
            out.push_str(before);
            out.push('<');
            rest = tail.get(1..).unwrap_or_default();
            continue;
        };

        out.push_str(before);
        out.push_str(tag.replacement());

        rest = if DISCARD_CONTENT.contains(&tag.name.as_str()) {
            skip_to_close(tag.rest, &tag.name)
        } else {
            tag.rest
        };
    }

    out.push_str(rest);
    out
}

struct Tag<'a> {
    name: String,
    closing: bool,
    rest: &'a str,
}

impl<'a> Tag<'a> {
    fn parse(input: &'a str) -> Option<Self> {
        let after = input.get(1..)?;
        let (closing, after) =
            after.strip_prefix('/').map_or((false, after), |tail| (true, tail));

        let mut chars = after.char_indices();
        let (_index, first) = chars.next()?;

        if !first.is_ascii_alphabetic() {
            return None;
        }

        let end = chars
            .find(|(_index, c)| !c.is_ascii_alphanumeric() && *c != '-')
            .map_or(after.len(), |(index, _c)| index);

        let name = after.get(..end)?.to_ascii_lowercase();
        let tail = after.get(end..)?;

        if !tail.starts_with(['>', '/', ' ', '\t', '\n', '\r']) {
            return None;
        }

        Some(Self { name, closing, rest: close_bracket(tail)? })
    }

    fn replacement(&self) -> &'static str {
        match self.name.as_str() {
            "br" => "\n",
            "hr" => "\n---\n",
            "p" | "div" | "li" | "tr" if self.closing => "\n",
            _ => "",
        }
    }
}

fn close_bracket(tail: &str) -> Option<&str> {
    let mut quote = None;

    for (index, c) in tail.char_indices() {
        match (quote, c) {
            (Some(open), c) if c == open => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            (None, '>') => return tail.get(index + 1..),
            (Some(_) | None, _) => {},
        }
    }

    None
}

fn skip_to_close<'a>(rest: &'a str, name: &str) -> &'a str {
    let needle = format!("</{name}");

    rest.to_ascii_lowercase().find(&needle).map_or("", |index| {
        rest.get(index..).and_then(close_bracket).unwrap_or_default()
    })
}

fn decode_entities(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(index) = rest.find('&') {
        let (before, tail) = rest.split_at_checked(index).unwrap_or((rest, ""));
        out.push_str(before);

        match entity(tail) {
            Some((decoded, tail)) => {
                out.push_str(&decoded);
                rest = tail;
            },
            None => {
                out.push('&');
                rest = tail.get(1..).unwrap_or_default();
            },
        }
    }

    out.push_str(rest);
    out
}

fn entity(input: &str) -> Option<(String, &str)> {
    let body = input.get(1..)?;
    let end = body.find(';').filter(|index| *index <= 10)?;
    let name = body.get(..end)?;
    let rest = body.get(end + 1..)?;

    if let Some(digits) = name.strip_prefix('#') {
        return numeric(digits).map(|c| (c, rest));
    }

    NAMED
        .iter()
        .find(|(entity, _replacement)| *entity == name)
        .map(|(_entity, replacement)| ((*replacement).to_owned(), rest))
}

fn numeric(digits: &str) -> Option<String> {
    let code = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse::<u32>().ok()?,
    };

    char::from_u32(code).map(|c| c.to_string())
}
