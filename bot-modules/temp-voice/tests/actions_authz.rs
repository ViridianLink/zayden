//! Server-side permission re-checks on the `actions` layer (temp-voice #4).
//!
//! Every temp-voice mutation is reachable from a component `custom_id`, which a
//! user can forge — so the M4 design's core claim is that each action re-checks
//! ownership/trust against the `VoiceChannelRow` itself rather than trusting the
//! interaction that carried it. That claim lives in a single private
//! `require_owner`/`require_trusted` line at the top of each action
//! (`src/actions/mod.rs`), and deleting or downgrading one of those lines
//! compiles, lints, and passes every other test in this crate.
//!
//! These tests pin the gate *mapping*: which of the two guards each action uses.
//! They assert the exact `PermissionError` variant rather than merely "an
//! error", because swapping `require_owner` for `require_trusted` still rejects
//! an outsider — the escalation only shows up when a *trusted* non-owner reaches
//! an owner-only action.
//!
//! Everything here is offline: every guard returns before its action's first
//! `.await`, so the `Http` and `PgPool` below are constructed but never dialled.

use std::error::Error;

use serenity::all::{ChannelId, GuildId, Http, UserId};
use serenity::secrets::Token;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use temp_voice::error::PermissionError;
use temp_voice::{TempVoiceError, VoiceChannelRow, VoiceStateCache, actions};

type TestResult = Result<(), Box<dyn Error>>;

const CHANNEL: ChannelId = ChannelId::new(100);
const GUILD: GuildId = GuildId::new(200);
const OWNER: UserId = UserId::new(1);
const TRUSTED: UserId = UserId::new(2);
const OUTSIDER: UserId = UserId::new(3);
const TARGET: UserId = UserId::new(4);

/// The two clients every action signature demands. Both are inert: the token is
/// well-formed but fake, and the pool is lazy, so neither opens a connection
/// unless an action gets past its guard — which is exactly what these tests
/// assert never happens.
struct Clients {
    http: Http,
    pool: PgPool,
}

fn clients() -> Result<Clients, Box<dyn Error>> {
    // `Token` only requires three non-empty dot-separated segments, so this
    // deliberately looks nothing like a real credential. Anything resembling
    // one trips GitHub push protection, even as a throwaway test fixture.
    let token: Token = "not-a-real.test.token".parse()?;

    Ok(Clients {
        http: Http::new(token),
        pool: PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")?,
    })
}

/// A channel owned by `OWNER` with `TRUSTED` on the trust list.
fn row() -> VoiceChannelRow {
    let mut row = VoiceChannelRow::new(CHANNEL, OWNER);
    row.trust(TRUSTED);
    row
}

#[track_caller]
fn assert_not_owner(result: &temp_voice::Result<String>) {
    assert!(
        matches!(
            result,
            Err(TempVoiceError::MissingPermissions(PermissionError::NotOwner))
        ),
        "expected the owner-only guard to reject the caller",
    );
}

#[track_caller]
fn assert_not_trusted(result: &temp_voice::Result<String>) {
    assert!(
        matches!(
            result,
            Err(TempVoiceError::MissingPermissions(PermissionError::NotTrusted))
        ),
        "expected the trusted-only guard to reject the caller",
    );
}

#[track_caller]
fn assert_user_is_owner(result: &temp_voice::Result<String>) {
    assert!(
        matches!(result, Err(TempVoiceError::UserIsOwner)),
        "the owner must not be able to claim their own channel",
    );
}

// ---------------------------------------------------------------------------
// Owner-gated actions: trust, password, transfer, delete.
//
// The `TRUSTED` caller is the load-bearing case — being on the trust list must
// not confer ownership. An `OUTSIDER` would be rejected either way.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn trust_rejects_trusted_non_owner() -> TestResult {
    let Clients { http, pool } = clients()?;

    assert_not_owner(
        &actions::trust(&http, &pool, CHANNEL, row(), TRUSTED, TARGET).await,
    );

    Ok(())
}

#[tokio::test]
async fn trust_rejects_outsider() -> TestResult {
    let Clients { http, pool } = clients()?;

    assert_not_owner(
        &actions::trust(&http, &pool, CHANNEL, row(), OUTSIDER, TARGET).await,
    );

    Ok(())
}

#[tokio::test]
async fn password_rejects_trusted_non_owner() -> TestResult {
    let Clients { http, pool } = clients()?;

    assert_not_owner(
        &actions::password(
            &http,
            &pool,
            GUILD,
            CHANNEL,
            row(),
            TRUSTED,
            "hunter2".to_string(),
        )
        .await,
    );

    Ok(())
}

#[tokio::test]
async fn password_rejects_outsider() -> TestResult {
    let Clients { http, pool } = clients()?;

    assert_not_owner(
        &actions::password(
            &http,
            &pool,
            GUILD,
            CHANNEL,
            row(),
            OUTSIDER,
            "hunter2".to_string(),
        )
        .await,
    );

    Ok(())
}

#[tokio::test]
async fn transfer_rejects_trusted_non_owner() -> TestResult {
    let Clients { http, pool } = clients()?;

    assert_not_owner(
        &actions::transfer(&http, &pool, CHANNEL, row(), TRUSTED, TARGET).await,
    );

    Ok(())
}

#[tokio::test]
async fn transfer_rejects_outsider() -> TestResult {
    let Clients { http, pool } = clients()?;

    assert_not_owner(
        &actions::transfer(&http, &pool, CHANNEL, row(), OUTSIDER, TARGET).await,
    );

    Ok(())
}

#[tokio::test]
async fn delete_rejects_trusted_non_owner() -> TestResult {
    let Clients { http, pool } = clients()?;

    assert_not_owner(&actions::delete(&http, &pool, CHANNEL, row(), TRUSTED).await);

    Ok(())
}

#[tokio::test]
async fn delete_rejects_outsider() -> TestResult {
    let Clients { http, pool } = clients()?;

    assert_not_owner(&actions::delete(&http, &pool, CHANNEL, row(), OUTSIDER).await);

    Ok(())
}

// ---------------------------------------------------------------------------
// Trusted-gated actions: kick, privacy, rename, limit, bitrate, region.
//
// Asserting `NotTrusted` (not merely "some error") is what pins these to
// `require_trusted`: if one were tightened to `require_owner`, the outsider
// would still be rejected, but with the other variant.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn kick_rejects_untrusted() -> TestResult {
    let Clients { http, .. } = clients()?;

    assert_not_trusted(&actions::kick(&http, GUILD, &row(), OUTSIDER, TARGET).await);

    Ok(())
}

#[tokio::test]
async fn privacy_rejects_untrusted() -> TestResult {
    let Clients { http, .. } = clients()?;

    assert_not_trusted(
        &actions::privacy(
            &http,
            GUILD,
            &VoiceStateCache::new(),
            CHANNEL,
            &row(),
            OUTSIDER,
            "lock",
        )
        .await,
    );

    Ok(())
}

#[tokio::test]
async fn rename_rejects_untrusted() -> TestResult {
    let Clients { http, .. } = clients()?;

    assert_not_trusted(
        &actions::rename(&http, CHANNEL, &row(), OUTSIDER, "hijacked".to_string())
            .await,
    );

    Ok(())
}

#[tokio::test]
async fn limit_rejects_untrusted() -> TestResult {
    let Clients { http, .. } = clients()?;

    assert_not_trusted(&actions::limit(&http, CHANNEL, &row(), OUTSIDER, 5).await);

    Ok(())
}

#[tokio::test]
async fn bitrate_rejects_untrusted() -> TestResult {
    let Clients { http, .. } = clients()?;

    assert_not_trusted(
        &actions::bitrate(&http, CHANNEL, &row(), OUTSIDER, 64).await,
    );

    Ok(())
}

#[tokio::test]
async fn region_rejects_untrusted() -> TestResult {
    let Clients { http, .. } = clients()?;

    assert_not_trusted(
        &actions::region(
            &http,
            CHANNEL,
            &row(),
            OUTSIDER,
            Some("rotterdam".to_string()),
        )
        .await,
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// `claim` is the eleventh mutation and deliberately uses neither guard — it is
// how a non-owner takes over an abandoned channel. Only its own first-statement
// invariant is reachable offline; the `OwnerInChannel` and `ClaimFailed` arms
// need the voice-state cache populated and a live pool respectively.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn claim_rejects_the_current_owner() -> TestResult {
    let Clients { http, pool } = clients()?;

    assert_user_is_owner(
        &actions::claim(
            &http,
            &pool,
            &VoiceStateCache::new(),
            CHANNEL,
            row(),
            OWNER,
        )
        .await,
    );

    Ok(())
}
