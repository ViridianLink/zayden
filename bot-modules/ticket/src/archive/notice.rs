use jiff::Timestamp;
use zayden_app::config::ARCHIVE_NEVER;

const SOLVED: &str = "This post has been marked as solved.";
const OPEN: &str = "-# Post closes <t:";
const CLOSE: &str = ":R>";

#[must_use]
pub fn deadline(now: Timestamp, archive_secs: i32) -> Option<i64> {
    (archive_secs != ARCHIVE_NEVER)
        .then(|| now.as_second() + i64::from(archive_secs))
}

#[must_use]
pub fn with_deadline(text: &str, deadline: Option<i64>) -> String {
    deadline
        .map_or_else(|| text.to_owned(), |at| format!("{text}\n{OPEN}{at}{CLOSE}"))
}

#[must_use]
pub fn solved_notice(deadline: Option<i64>) -> String {
    with_deadline(SOLVED, deadline)
}

#[must_use]
pub fn parse(content: &str) -> Option<i64> {
    let start = content.rfind(OPEN)? + OPEN.len();
    let rest = content.get(start..)?;
    let end = rest.find(CLOSE)?;

    rest.get(..end)?.parse().ok()
}
