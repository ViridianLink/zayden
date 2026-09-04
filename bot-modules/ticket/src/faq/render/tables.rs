const LINE_BUDGET: usize = 56;
const MAX_COLUMNS: usize = 5;

pub(crate) fn reflow(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rows: Vec<Vec<String>> = Vec::new();

    for line in content.lines() {
        match parse_row(line) {
            Some(row) => rows.push(row),
            None => {
                flush(&mut out, &mut rows);
                out.push_str(line);
                out.push('\n');
            },
        }
    }

    flush(&mut out, &mut rows);
    out
}

fn flush(out: &mut String, rows: &mut Vec<Vec<String>>) {
    let rows = std::mem::take(rows);

    let Some((header, body)) = split_header(&rows) else {
        for row in &rows {
            out.push_str(&row.join(" | "));
            out.push('\n');
        }
        return;
    };

    let widths = widths(header, body);
    let total = widths.iter().sum::<usize>() + widths.len().saturating_sub(1) * 3;

    if widths.len() <= MAX_COLUMNS && total <= LINE_BUDGET {
        grid(out, header, body, &widths);
    } else {
        list(out, header, body);
    }
}

fn split_header(rows: &[Vec<String>]) -> Option<(&Vec<String>, &[Vec<String>])> {
    let header = rows.first()?;
    let delimiter = rows.get(1)?;
    let body = rows.get(2..)?;

    let dashes = delimiter.iter().all(|cell| {
        !cell.is_empty() && cell.chars().all(|c| matches!(c, '-' | ':' | ' '))
    });

    (dashes && !body.is_empty() && header.len() > 1).then_some((header, body))
}

fn widths(header: &[String], body: &[Vec<String>]) -> Vec<usize> {
    let mut widths =
        header.iter().map(|cell| cell.chars().count()).collect::<Vec<_>>();

    for row in body {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index) {
                *width = (*width).max(cell.chars().count());
            }
        }
    }

    widths
}

fn grid(
    out: &mut String,
    header: &[String],
    body: &[Vec<String>],
    widths: &[usize],
) {
    out.push_str("```\n");
    push_padded(out, header, widths);

    let rule = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join("-+-");
    out.push_str(&rule);
    out.push('\n');

    for row in body {
        push_padded(out, row, widths);
    }

    out.push_str("```\n");
}

fn push_padded(out: &mut String, row: &[String], widths: &[usize]) {
    let cells = widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let cell = row.get(index).map_or("", String::as_str);
            let pad = width.saturating_sub(cell.chars().count());

            format!("{cell}{}", " ".repeat(pad))
        })
        .collect::<Vec<_>>()
        .join(" | ");

    out.push_str(cells.trim_end());
    out.push('\n');
}

fn list(out: &mut String, header: &[String], body: &[Vec<String>]) {
    for row in body {
        let Some(first) = row.first().filter(|cell| !cell.is_empty()) else {
            continue;
        };

        out.push_str("**");
        out.push_str(first);
        out.push_str("**\n");

        for (index, cell) in row.iter().enumerate().skip(1) {
            if cell.is_empty() {
                continue;
            }

            out.push_str("- ");
            if let Some(key) = header.get(index).filter(|key| !key.is_empty()) {
                out.push_str(key);
                out.push_str(": ");
            }
            out.push_str(cell);
            out.push('\n');
        }

        out.push('\n');
    }
}

fn parse_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();

    if !trimmed.starts_with('|') || !trimmed.contains('|') {
        return None;
    }

    let inner = trimmed.trim_start_matches('|').trim_end_matches('|');

    let cells = split_cells(inner)
        .map(|cell| cell.trim().replace("<br>", " ").replace("\\|", "|"))
        .collect::<Vec<_>>();

    (cells.len() > 1).then_some(cells)
}

fn split_cells(inner: &str) -> impl Iterator<Item = &str> {
    let mut rest = Some(inner);

    std::iter::from_fn(move || {
        let current = rest?;
        let mut from = 0;

        loop {
            let Some(index) = current.get(from..)?.find('|').map(|i| from + i)
            else {
                rest = None;
                return Some(current);
            };

            let escaped =
                current.get(..index).is_some_and(|before| before.ends_with('\\'));

            if !escaped {
                rest = current.get(index + 1..);
                return current.get(..index);
            }

            from = index + 1;
        }
    })
}
