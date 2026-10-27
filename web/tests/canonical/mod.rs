use std::iter::Peekable;
use std::str::Chars;

/// `html` with the attributes of every start tag sorted by name, so markup
/// from components that spread attributes compares independently of order.
pub(crate) fn canonical(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '<' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('!') => {
                let end = comment_end(&chars);
                copy_through(&mut chars, &mut out, "<", end);
            },
            Some('/' | '?') => copy_through(&mut chars, &mut out, "<", ">"),
            Some(next) if next.is_ascii_alphabetic() => {
                let name = start_tag(&mut chars, &mut out);
                if name == "script" || name == "style" {
                    copy_through(&mut chars, &mut out, "", &format!("</{name}>"));
                }
            },
            _ => out.push(c),
        }
    }

    out
}

fn comment_end(chars: &Peekable<Chars<'_>>) -> &'static str {
    let rest: String = chars.clone().take(3).collect();
    if rest == "!--" { "-->" } else { ">" }
}

/// Appends `prefix`, then every char up to and including `end` (or the rest).
fn copy_through(
    chars: &mut Peekable<Chars<'_>>,
    out: &mut String,
    prefix: &str,
    end: &str,
) {
    out.push_str(prefix);
    let mut copied = String::new();
    for c in chars.by_ref() {
        copied.push(c);
        if copied.ends_with(end) {
            break;
        }
    }
    out.push_str(&copied);
}

/// Reads a start tag after its `<` and writes it with sorted attributes.
/// Returns the tag name.
fn start_tag(chars: &mut Peekable<Chars<'_>>, out: &mut String) -> String {
    let name = take_while(chars, |c| !c.is_whitespace() && c != '>' && c != '/');
    let mut attributes: Vec<(String, Option<String>)> = Vec::new();
    let mut self_closing = false;

    loop {
        take_while(chars, char::is_whitespace);
        match chars.peek() {
            None => break,
            Some('>') => {
                chars.next();
                break;
            },
            Some('/') => {
                chars.next();
                self_closing = true;
            },
            Some(_) => {
                let key = take_while(chars, |c| {
                    !c.is_whitespace() && c != '=' && c != '>' && c != '/'
                });
                let value = if chars.peek() == Some(&'=') {
                    chars.next();
                    Some(if chars.peek() == Some(&'"') {
                        chars.next();
                        let value = take_while(chars, |c| c != '"');
                        chars.next();
                        value
                    } else {
                        take_while(chars, |c| !c.is_whitespace() && c != '>')
                    })
                } else {
                    None
                };
                attributes.push((key, value));
            },
        }
    }

    attributes.sort_by(|(a, _), (b, _)| a.cmp(b));
    out.push('<');
    out.push_str(&name);
    for (key, value) in attributes {
        out.push(' ');
        out.push_str(&key);
        if let Some(value) = value {
            out.push_str("=\"");
            out.push_str(&value);
            out.push('"');
        }
    }
    out.push_str(if self_closing { "/>" } else { ">" });

    name
}

fn take_while(
    chars: &mut Peekable<Chars<'_>>,
    keep: impl Fn(char) -> bool,
) -> String {
    let mut taken = String::new();
    while let Some(&c) = chars.peek() {
        if !keep(c) {
            break;
        }
        taken.push(c);
        chars.next();
    }
    taken
}
