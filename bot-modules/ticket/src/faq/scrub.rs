use std::sync::LazyLock;

use regex::Regex;
use tracing::error;

const REDACTED: &str = "[redacted]";

const PATTERNS: &[(&str, &str)] = &[
    (r"([a-zA-Z][a-zA-Z0-9+.\-]*://)[^\s/@:]+:[^\s/@]+@", "${1}[redacted]@"),
    (r"\beyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}\b", REDACTED),
    (
        r"\b(?:sk|pk|rk|xoxb|xoxp|xoxa|xoxs|ghp|gho|ghs|ghu|github_pat)[-_][A-Za-z0-9_\-]{16,}\b",
        REDACTED,
    ),
    (r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}", REDACTED),
    (r"\[[0-9a-fA-F:]{2,}\]", REDACTED),
    (r"\b(?:[0-9a-fA-F]{1,4}:){2,7}[0-9a-fA-F]{1,4}\b", REDACTED),
    (
        r"\b(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])\b",
        REDACTED,
    ),
    (r"<@[!&]?[0-9]+>", REDACTED),
    (r"\b[0-9]{17,20}\b", REDACTED),
    (r"\b[0-9a-fA-F]{32,}\b", REDACTED),
];

static COMPILED: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    PATTERNS
        .iter()
        .filter_map(|(pattern, replacement)| match Regex::new(pattern) {
            Ok(regex) => Some((regex, *replacement)),
            Err(e) => {
                error!(pattern, error = ?e, "faq scrub pattern failed to compile");
                None
            },
        })
        .collect()
});

#[must_use]
pub fn redact(text: &str) -> String {
    COMPILED.iter().fold(text.to_owned(), |acc, (regex, replacement)| {
        regex.replace_all(&acc, *replacement).into_owned()
    })
}
