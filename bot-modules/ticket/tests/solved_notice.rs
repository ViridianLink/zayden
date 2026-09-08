//! The "post closes" line on `/solved`.
//!
//! Marking a ticket solved does not close it there and then - `mark_solved`
//! schedules the archive for `solved_archive_secs` later. The notice has to
//! name that moment, or Discord renders `<t:...:R>` as "in 0 seconds" and the
//! reporter reads a post that is still open as already gone.
//!
//! The line is also the schedule: nothing else records the deadline, so a
//! restart rebuilds the archive by parsing this back out of the thread. The
//! round trip is what these tests hold together.

use jiff::Timestamp;
use ticket::archive::notice::{deadline, parse, solved_notice, with_deadline};
use zayden_app::config::ARCHIVE_NEVER;

const NOW: i64 = 1_800_000_000;

fn now() -> Timestamp {
    Timestamp::from_second(NOW).unwrap_or(Timestamp::UNIX_EPOCH)
}

#[test]
fn the_close_stamp_is_the_archive_deadline_not_the_moment_of_solving() {
    let notice = solved_notice(deadline(now(), 600));

    assert_eq!(
        notice,
        "This post has been marked as solved.\n-# Post closes <t:1800000600:R>"
    );
}

#[test]
fn a_thread_that_never_archives_makes_no_promise_about_closing() {
    assert_eq!(
        solved_notice(deadline(now(), ARCHIVE_NEVER)),
        "This post has been marked as solved."
    );
}

#[test]
fn every_configurable_delay_lands_in_the_future() {
    for secs in [1_i32, 60, 3_600, 86_400] {
        let stamp = parse(&solved_notice(deadline(now(), secs))).unwrap_or_default();

        assert!(stamp > NOW, "{secs}s produced a stamp at or before now");
    }
}

/// The whole point of the line: what the solve wrote is what the restart reads.
#[test]
fn the_deadline_survives_the_round_trip() {
    let at = deadline(now(), 3_600);

    assert_eq!(parse(&solved_notice(at)), at);
}

/// The button path appends the line to its own wording, so the parser cannot
/// depend on the notice's first line.
#[test]
fn any_message_carrying_the_line_is_readable() {
    let at = deadline(now(), 600);

    let content = with_deadline("Ada marked this solved. Thanks!", at);

    assert_eq!(parse(&content), at);
}

#[test]
fn a_post_with_no_closing_line_names_no_deadline() {
    assert_eq!(parse("This post has been marked as solved."), None);
    assert_eq!(parse("Ticket reopened"), None);
    assert_eq!(parse(""), None);
}

/// A post solved, reopened and solved again carries two lines; the later one
/// is the live deadline.
#[test]
fn the_last_line_in_a_message_wins() {
    let content = format!(
        "{}\n{}",
        with_deadline("first", Some(1_800_000_600)),
        with_deadline("second", Some(1_800_003_600))
    );

    assert_eq!(parse(&content), Some(1_800_003_600));
}

#[test]
fn a_malformed_stamp_is_not_a_deadline() {
    assert_eq!(parse("-# Post closes <t:soon:R>"), None);
    assert_eq!(parse("-# Post closes <t:1800000600"), None);
}
