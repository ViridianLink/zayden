//! The honeypot's per-offender action guard.
//!
//! `message_create` calls `GUARD.claim` to hold exactly one invariant, stated in
//! the comment directly above the call site: *"A flood arrives faster than the
//! ban lands; act once per offender."*
//!
//! A flood is not an edge case here, it is the crate's **design case**. Serenity
//! spawns a task per gateway event (`spawn_named("dispatch::user", …)` in its
//! `gateway/client/dispatch.rs`), so a spam bot posting N messages into the
//! decoy channel in one tick runs N `message_create` futures concurrently
//! against a single `(guild, user)` key. Every extra winner is another
//! `ban` + `unban` pair against Discord's rate limiter at the worst possible
//! moment, another duplicate `SoftBan` row in the infraction log, and another
//! chance for the `unban` half to fail and leave a standing ban (honeypot #1).
//!
//! These run against the production `GUARD` static rather than a private
//! constructor, so they widen no API. That makes key disjointness this file's
//! one invariant: **every test owns its own guild id**, so no two tests can ever
//! contend on the same cache key.

use std::sync::Arc;

use honeypot::guard::GUARD;
use serenity::all::{GuildId, UserId};
use tokio::sync::Barrier;

const G_SEQUENTIAL: u64 = 900_000_000_000_000_001;
const G_RELEASE: u64 = 900_000_000_000_000_002;
const G_USERS: u64 = 900_000_000_000_000_003;
const G_GUILDS_A: u64 = 900_000_000_000_000_004;
const G_GUILDS_B: u64 = 900_000_000_000_000_005;
const G_FLOOD: u64 = 900_000_000_000_000_006;

const OFFENDER: u64 = 800_000_000_000_000_001;

/// Racers per key. Each stands for one message of the flood, i.e. one
/// `message_create` task.
const RACERS: usize = 16;
/// Independent keys the race is repeated over. A single key would make the
/// assertion a coin flip on scheduling; over this many, a guard that does not
/// serialise cannot plausibly come out clean.
const KEYS: u64 = 128;

#[tokio::test]
async fn a_second_claim_on_the_same_key_is_refused() {
    let guild = GuildId::new(G_SEQUENTIAL);
    let user = UserId::new(OFFENDER);

    assert!(GUARD.claim(guild, user).await, "the first claim must win");
    assert!(!GUARD.claim(guild, user).await, "the second must be refused");
}

#[tokio::test]
async fn release_allows_a_fresh_claim() {
    let guild = GuildId::new(G_RELEASE);
    let user = UserId::new(OFFENDER);

    assert!(GUARD.claim(guild, user).await);
    GUARD.release(guild, user).await;

    // `message_create` releases on every early return after a successful claim
    // (facts lookup failed, member was exempt, the ban itself failed), so a
    // later message from the same user must be actionable again.
    assert!(
        GUARD.claim(guild, user).await,
        "release must make the offender claimable again"
    );
}

#[tokio::test]
async fn distinct_offenders_do_not_share_a_claim() {
    let guild = GuildId::new(G_USERS);

    assert!(GUARD.claim(guild, UserId::new(OFFENDER)).await);
    assert!(
        GUARD.claim(guild, UserId::new(OFFENDER + 1)).await,
        "a second offender in the same guild must be actioned independently"
    );
}

#[tokio::test]
async fn distinct_guilds_do_not_share_a_claim() {
    let user = UserId::new(OFFENDER);

    assert!(GUARD.claim(GuildId::new(G_GUILDS_A), user).await);
    assert!(
        GUARD.claim(GuildId::new(G_GUILDS_B), user).await,
        "the same account raiding two guilds must be actioned in both"
    );
}

/// The regression test for honeypot #2.
///
/// Fails against the check-then-act implementation (`get().await` then
/// `insert().await`, with the key unguarded in between): concurrent racers all
/// observe the miss and all claim. Passes once the claim is a single atomic
/// per-key operation.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn a_flood_on_one_key_yields_exactly_one_winner() {
    let guild = GuildId::new(G_FLOOD);

    let mut total_winners = 0usize;
    let mut worst_key: Option<(u64, usize)> = None;
    let mut join_failures = 0usize;

    for key in 0..KEYS {
        // One distinct offender per round, so a round cannot inherit the
        // previous round's cache entry.
        let user = UserId::new(OFFENDER + 100 + key);

        // Release every racer at once — without this they queue behind each
        // other's spawn and the window closes on its own.
        let gate = Arc::new(Barrier::new(RACERS));

        let mut racers = Vec::with_capacity(RACERS);
        for _ in 0..RACERS {
            let gate = Arc::clone(&gate);
            racers.push(tokio::spawn(async move {
                gate.wait().await;
                GUARD.claim(guild, user).await
            }));
        }

        let mut winners = 0usize;
        for racer in racers {
            match racer.await {
                Ok(true) => winners += 1,
                Ok(false) => {},
                Err(_) => join_failures += 1,
            }
        }

        total_winners += winners;
        if winners > worst_key.map_or(0, |(_, w)| w) {
            worst_key = Some((key, winners));
        }
    }

    assert_eq!(join_failures, 0, "no racer task may panic");

    let expected = usize::try_from(KEYS).unwrap_or(usize::MAX);
    assert_eq!(
        total_winners, expected,
        "each of the {KEYS} floods must produce exactly one winner, got \
         {total_winners} across all keys (worst key: {worst_key:?}). More than \
         one winner means `claim` let concurrent callers past the same key — \
         every extra winner is a duplicate ban+unban pair and a duplicate \
         SoftBan record."
    );
}
