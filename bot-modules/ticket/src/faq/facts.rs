use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;
use tracing::error;

const FENCES: [&str; 2] = ["```", "~~~"];

const PATTERNS: &[&str] = &[
    r"`([^`\n]+)`",
    r"(https?://[^\s<>()\[\]`]+[^\s<>()\[\]`.,;:!?'\x22])",
    r"\b(v?[0-9]+(?:\.[0-9]+)+)\b",
];

static COMPILED: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    PATTERNS
        .iter()
        .filter_map(|pattern| match Regex::new(pattern) {
            Ok(regex) => Some(regex),
            Err(e) => {
                error!(pattern, error = ?e, "faq fact pattern failed to compile");
                None
            },
        })
        .collect()
});

#[must_use]
pub fn literals(markdown: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut in_fence = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        if FENCES.iter().any(|fence| trimmed.starts_with(fence)) {
            in_fence = !in_fence;
            continue;
        }

        if in_fence {
            insert(&mut found, trimmed);
            continue;
        }

        for regex in COMPILED.iter() {
            for captures in regex.captures_iter(line) {
                if let Some(capture) = captures.get(1) {
                    insert(&mut found, capture.as_str());
                }
            }
        }
    }

    found
}

#[must_use]
pub fn missing<'a>(literals: &'a BTreeSet<String>, text: &str) -> Vec<&'a str> {
    let haystack = collapse(text);

    literals
        .iter()
        .filter(|literal| !haystack.contains(literal.as_str()))
        .map(String::as_str)
        .collect()
}

fn insert(found: &mut BTreeSet<String>, literal: &str) {
    let literal = collapse(literal);

    if !literal.is_empty() {
        found.insert(literal);
    }
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
