//! The "post closes" line on `/solved`.
//!
//! Marking a ticket solved does not close it there and then - `mark_solved`
//! schedules the archive for `solved_archive_secs` later. The notice has to
//! name that moment, or Discord renders `<t:...:R>` as "in 0 seconds" and the
//! reporter reads a post that is still open as already gone.

use jiff::Timestamp;
use ticket::solve::solved_notice;
use zayden_app::config::ARCHIVE_NEVER;

const NOW: i64 = 1_800_000_000;

fn now() -> Timestamp {
    Timestamp::from_second(NOW).unwrap_or(Timestamp::UNIX_EPOCH)
}

#[test]
fn the_close_stamp_is_the_archive_deadline_not_the_moment_of_solving() {
    let notice = solved_notice(now(), 600);

    assert_eq!(
        notice,
        "This post has been marked as solved.\n-# Post closes <t:1800000600:R>"
    );
}

#[test]
fn a_thread_that_never_archives_makes_no_promise_about_closing() {
    assert_eq!(
        solved_notice(now(), ARCHIVE_NEVER),
        "This post has been marked as solved."
    );
}

#[test]
fn every_configurable_delay_lands_in_the_future() {
    for secs in [1_i32, 60, 3_600, 86_400] {
        let notice = solved_notice(now(), secs);

        let stamp = notice
            .rsplit_once("<t:")
            .and_then(|(_, tail)| tail.split_once(':'))
            .and_then(|(stamp, _)| stamp.parse::<i64>().ok())
            .unwrap_or_default();

        assert!(stamp > NOW, "{secs}s produced a stamp at or before now");
    }
}
