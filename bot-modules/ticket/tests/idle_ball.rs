//! The ball is stored as one boolean, so the mapping has to be exact.
//!
//! It is named for the side that owes a reply rather than the side that spoke
//! last, because "Still need help" moves the ball with nobody speaking.

use ticket::Ball;

#[test]
fn the_column_round_trips() {
    for ball in [Ball::Op, Ball::Helper] {
        assert_eq!(Ball::from_column(ball.column()), ball);
    }
}

/// A fresh row defaults `waiting_on_helper` to true, so a ticket nobody has
/// answered is waiting on the helpers - not on the person who just opened it.
#[test]
fn a_fresh_ticket_waits_on_the_helpers() {
    assert_eq!(Ball::from_column(true), Ball::Helper);
    assert_eq!(Ball::from_column(false), Ball::Op);
}
