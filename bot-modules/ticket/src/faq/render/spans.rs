#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Span<'a> {
    Text(&'a str),
    Code(&'a str),
}

impl<'a> Span<'a> {
    pub(crate) const fn as_str(self) -> &'a str {
        match self {
            Self::Text(s) | Self::Code(s) => s,
        }
    }
}

pub(crate) fn map_text(content: &str, mut f: impl FnMut(&str) -> String) -> String {
    let mut out = String::with_capacity(content.len());

    for span in split(content) {
        match span {
            Span::Text(text) => out.push_str(&f(text)),
            Span::Code(code) => out.push_str(code),
        }
    }

    out
}

pub(crate) fn blocks(content: &str) -> Vec<Span<'_>> {
    split_fences(content)
}

pub(crate) fn split(content: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();

    for block in split_fences(content) {
        match block {
            Span::Code(_) => spans.push(block),
            Span::Text(text) => split_inline(text, &mut spans),
        }
    }

    spans
}

fn split_fences(content: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let mut text_start = 0;
    let mut fence: Option<(char, usize, usize)> = None;

    for (start, line) in lines(content) {
        let end = start + line.len();

        match fence {
            None => {
                if let Some((marker, width)) = fence_marker(line) {
                    push(&mut spans, Span::Text, content, text_start, start);
                    fence = Some((marker, width, start));
                }
            },
            Some((marker, width, open)) => {
                if closes(line, marker, width) {
                    push(&mut spans, Span::Code, content, open, end);
                    text_start = end;
                    fence = None;
                }
            },
        }
    }

    match fence {
        Some((_marker, _width, open)) => {
            push(&mut spans, Span::Code, content, open, content.len());
        },
        None => push(&mut spans, Span::Text, content, text_start, content.len()),
    }

    spans
}

fn split_inline<'a>(text: &'a str, spans: &mut Vec<Span<'a>>) {
    let bytes = text.as_bytes();
    let mut text_start = 0;
    let mut cursor = 0;

    while cursor < bytes.len() {
        let Some(open) = find_backtick(bytes, cursor) else { break };
        let width = run_width(bytes, open);

        match closing_run(bytes, open + width, width) {
            Some(close) => {
                push(spans, Span::Text, text, text_start, open);
                push(spans, Span::Code, text, open, close + width);
                text_start = close + width;
                cursor = text_start;
            },
            None => cursor = open + width,
        }
    }

    push(spans, Span::Text, text, text_start, text.len());
}

fn push<'a>(
    spans: &mut Vec<Span<'a>>,
    kind: fn(&'a str) -> Span<'a>,
    content: &'a str,
    start: usize,
    end: usize,
) {
    if start >= end {
        return;
    }

    if let Some(slice) = content.get(start..end) {
        spans.push(kind(slice));
    }
}

fn lines(content: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut start = 0;

    std::iter::from_fn(move || {
        if start >= content.len() {
            return None;
        }

        let rest = content.get(start..)?;
        let len = rest.find('\n').map_or(rest.len(), |index| index + 1);
        let line = rest.get(..len)?;
        let offset = start;
        start += len;

        Some((offset, line))
    })
}

fn fence_marker(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start_matches(' ');

    if line.len() - trimmed.len() > 3 {
        return None;
    }

    let marker = trimmed.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let width = trimmed.chars().take_while(|c| *c == marker).count();

    (width >= 3).then_some((marker, width))
}

fn closes(line: &str, marker: char, width: usize) -> bool {
    let Some((found, found_width)) = fence_marker(line) else {
        return false;
    };

    found == marker
        && found_width >= width
        && line.trim().chars().all(|c| c == marker)
}

fn find_backtick(bytes: &[u8], from: usize) -> Option<usize> {
    bytes.get(from..)?.iter().position(|b| *b == b'`').map(|index| from + index)
}

fn run_width(bytes: &[u8], from: usize) -> usize {
    bytes
        .get(from..)
        .map_or(0, |rest| rest.iter().take_while(|b| **b == b'`').count())
}

fn closing_run(bytes: &[u8], from: usize, width: usize) -> Option<usize> {
    let mut cursor = from;

    while let Some(candidate) = find_backtick(bytes, cursor) {
        let found = run_width(bytes, candidate);

        if found == width {
            return Some(candidate);
        }

        cursor = candidate + found;
    }

    None
}
