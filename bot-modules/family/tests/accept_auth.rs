//! Regression tests for the self-accept crash on the marry/adopt proposal
//! buttons.
//!
//! The proposal message reads "@target, @author wants to marry you!", so it
//! mentions the **author** as well as the target. The accept handlers authorised
//! on "is mentioned", which the author satisfies, and additionally short-
//! circuited on `responder == author` — so an author clicking their own Accept
//! button passed the guard and married/adopted themselves. `FamilyRow::save`
//! then wrote a row with both id columns equal, which Postgres rejected:
//!
//! ```text
//! new row for relation "family_partners" violates check constraint
//! "family_partners_check"    -- CHECK (user_id < partner_id)
//! ```
//!
//! surfacing to the user as an opaque internal error rather than "You can't
//! marry yourself!". The end-to-end accept paths need a live `PgPool` plus a
//! Discord interaction (see the note in `invariants.rs`), so these tests pin the
//! pure authorisation decision the handlers now delegate to.

use family::components::{AcceptAuth, accept_auth};

const AUTHOR: u64 = 1_262_104_550_207_258_686;
const TARGET: u64 = 1_262_104_550_207_258_687;
const BYSTANDER: u64 = 1_262_104_550_207_258_688;

#[test]
fn author_cannot_accept_their_own_proposal() {
    // The crashing case: the author is mentioned by the proposal text, so the
    // mention check alone would let them through.
    assert_eq!(
        accept_auth(AUTHOR.into(), AUTHOR.into(), true),
        AcceptAuth::SelfAccept
    );
}

#[test]
fn author_is_rejected_even_when_not_mentioned() {
    // Self-accept is rejected on identity, independently of the mention list.
    assert_eq!(
        accept_auth(AUTHOR.into(), AUTHOR.into(), false),
        AcceptAuth::SelfAccept
    );
}

#[test]
fn mentioned_target_may_accept() {
    assert_eq!(accept_auth(AUTHOR.into(), TARGET.into(), true), AcceptAuth::Allowed);
}

#[test]
fn unmentioned_bystander_may_not_accept() {
    assert_eq!(
        accept_auth(AUTHOR.into(), BYSTANDER.into(), false),
        AcceptAuth::Unauthorised
    );
}
