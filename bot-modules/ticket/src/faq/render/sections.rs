use crate::faq::render::spans::{Span, blocks};
use crate::faq::render::truncate::{ELLIPSIS, close_fence, truncate};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Section {
    pub level: usize,
    pub title: String,
    pub anchor: String,
    pub body: String,
}

impl Section {
    #[must_use]
    pub fn render(&self) -> String {
        if self.title.is_empty() {
            return self.body.trim().to_owned();
        }

        let heading = match self.level {
            0 | 1 => format!("## {}", self.title),
            2 => format!("### {}", self.title),
            _ => format!("**{}**", self.title),
        };

        if self.body.trim().is_empty() {
            heading
        } else {
            format!("{heading}\n{}", self.body.trim())
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.title.is_empty() && self.body.trim().is_empty()
    }
}

#[must_use]
pub fn split_sections(content: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current = Section {
        level: 0,
        title: String::new(),
        anchor: String::new(),
        body: String::new(),
    };

    for (line, in_code) in code_aware_lines(content) {
        match heading(line).filter(|_h| !in_code) {
            Some((level, title)) => {
                if !current.is_empty() {
                    sections.push(std::mem::take(&mut current));
                }

                current = Section {
                    level,
                    anchor: anchor(&title),
                    title,
                    body: String::new(),
                };
            },
            None => {
                current.body.push_str(line);
                current.body.push('\n');
            },
        }
    }

    if !current.is_empty() {
        sections.push(current);
    }

    sections
}

#[must_use]
pub fn fit(sections: &[Section], limit: usize) -> String {
    let reserved = limit.saturating_sub(ELLIPSIS.chars().count());
    let mut out = String::new();

    for (index, section) in sections.iter().enumerate() {
        let rendered = section.render();
        let addition = rendered.chars().count() + if out.is_empty() { 0 } else { 2 };
        let budget = if index + 1 == sections.len() { limit } else { reserved };

        if out.chars().count() + addition > budget {
            if out.is_empty() {
                return truncate(&rendered, limit);
            }

            close_fence(&mut out);
            out.push_str(ELLIPSIS);
            return out;
        }

        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&rendered);
    }

    out
}

#[must_use]
pub fn best_match<'a>(sections: &'a [Section], query: &str) -> Option<&'a Section> {
    let terms = terms(query);

    if terms.is_empty() {
        return sections.first();
    }

    sections
        .iter()
        .map(|section| (score(section, &terms), section))
        .max_by_key(|(score, _section)| *score)
        .filter(|(score, _section)| *score > 0)
        .map(|(_score, section)| section)
        .or_else(|| sections.first())
}

fn score(section: &Section, terms: &[String]) -> usize {
    let title = section.title.to_lowercase();
    let body = section.body.to_lowercase();

    terms
        .iter()
        .map(|term| {
            usize::from(title.contains(term.as_str())) * 8
                + body.matches(term.as_str()).count().min(4)
        })
        .sum()
}

fn terms(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.chars().count() > 2)
        .map(str::to_lowercase)
        .collect()
}

#[must_use]
pub fn anchor(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut dash = false;

    for c in title.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            dash = false;
        } else if !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }

    out.trim_end_matches('-').to_owned()
}

fn heading(line: &str) -> Option<(usize, String)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();

    if !(1..=6).contains(&hashes) {
        return None;
    }

    let rest = line.get(hashes..)?;

    if !rest.starts_with(' ') {
        return None;
    }

    let title = rest.trim().trim_end_matches('#').trim();

    (!title.is_empty()).then(|| (hashes, title.to_owned()))
}

fn code_aware_lines(content: &str) -> Vec<(&str, bool)> {
    let mut out = Vec::new();

    for span in blocks(content) {
        let in_code = matches!(span, Span::Code(_));

        for line in span.as_str().lines() {
            out.push((line, in_code));
        }
    }

    out
}
