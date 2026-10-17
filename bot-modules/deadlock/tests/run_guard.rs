use std::sync::Arc;

use deadlock::cron::RunGuard;

#[test]
fn only_one_run_holds_the_guard_at_a_time() {
    let guard = Arc::new(RunGuard::default());

    let ticket = guard.try_start();
    assert!(ticket.is_some());
    assert!(guard.try_start().is_none());

    drop(ticket);
    assert!(guard.try_start().is_some());
}
