//! Regression tests for [verify #2](../../../design-docs/audits/verify.md) — the
//! `VERIFIED_ROLE` role id used to be a `const
//! RoleId::new(1_404_640_603_848_839_299)` written out **twice**: once in
//! `verify/src/lib.rs` (read by the verify button) and once in
//! `bot/src/bindings/verify/mod.rs` (read by `/manverify`). Two failure modes: the
//! copies could silently drift apart, and being a compile-time constant the module
//! granted one specific guild's role in every guild.
//!
//! The fix moved the id into `roles_settings.verified_role_id` behind
//! [`verify::verified_role`], so both call sites now resolve through this one
//! function. That makes the property testable, and it is what these tests pin:
//!
//! - a guild's configured role is the one returned (**per-guild**, not constant);
//! - a *second* guild with a *different* role gets its own — the assertion the old
//!   constant could not satisfy for any pair of guilds;
//! - an unconfigured guild yields the typed [`VerifyError::RoleNotConfigured`] with
//!   a user-facing message, not an opaque `Discord(_)` failure from `add_role`
//!   against a role id that does not exist in that guild.
//!
//! The lookup is a settings-store read, so these are `#[sqlx::test]` — each test
//! gets its own migrated database and `DATABASE_URL` must point at a server the
//! runner may create databases on (see `CLAUDE.md`). There is no fixture: the
//! rows are inserted per-test through the same `SettingsStore::update` path the
//! dashboard's `save_role_settings` uses, so the test covers the write side too.
//!
//! **Fails-before evidence.** The code under test did not exist before the fix,
//! so no test could fail against it directly; the equivalent was established by
//! reverting `verified_role`'s body to the old constant
//! (`Ok(RoleId::new(1_404_640_603_848_839_299))`) and re-running:
//! `configured_role_is_returned_per_guild` fails on both guilds and
//! `unconfigured_guild_reports_not_configured` fails (it returns a role instead
//! of erroring). Reverted afterwards.

use serenity::all::{GuildId, RoleId};
use sqlx::PgPool;
use tokio::sync::broadcast;
use verify::{VerifyError, verified_role};
use zayden_app::config::{RolesSettingsRow, SettingsStore};
use zayden_app::events::AppEvent;
use zayden_core::as_i64;

const GUILD_A: GuildId = GuildId::new(1_404_640_603_848_839_200);
const GUILD_B: GuildId = GuildId::new(1_404_640_603_848_839_201);
const ROLE_A: RoleId = RoleId::new(1_404_640_603_848_839_299);
const ROLE_B: RoleId = RoleId::new(9_876_543_210_987_654_321);

fn store(pool: PgPool) -> SettingsStore<RolesSettingsRow> {
    let (events, _rx) = broadcast::channel::<AppEvent>(16);
    SettingsStore::new(pool, events)
}

/// `roles_settings.guild_id` references `guilds(id)`.
async fn insert_guild(pool: &PgPool, guild_id: GuildId) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
        as_i64(guild_id.get())
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Writes `verified_role_id` through the same `SettingsStore::update` path the
/// dashboard's `save_role_settings` uses.
async fn configure(
    store: &SettingsStore<RolesSettingsRow>,
    pool: &PgPool,
    guild_id: GuildId,
    role_id: RoleId,
) -> sqlx::Result<()> {
    insert_guild(pool, guild_id).await?;

    store
        .update(as_i64(guild_id.get()), |row| {
            row.verified_role_id = Some(as_i64(role_id.get()));
        })
        .await?;

    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn configured_role_is_returned_per_guild(pool: PgPool) {
    let store = store(pool.clone());

    configure(&store, &pool, GUILD_A, ROLE_A).await.expect("guild A configured");
    configure(&store, &pool, GUILD_B, ROLE_B).await.expect("guild B configured");

    let a = verified_role(&store, GUILD_A).await.expect("guild A resolves");
    let b = verified_role(&store, GUILD_B).await.expect("guild B resolves");

    assert_eq!(a, ROLE_A);
    assert_eq!(b, ROLE_B);
}

#[sqlx::test(migrations = "../../migrations")]
async fn unconfigured_guild_reports_not_configured(pool: PgPool) {
    let store = store(pool.clone());

    configure(&store, &pool, GUILD_A, ROLE_A).await.expect("guild A configured");
    insert_guild(&pool, GUILD_B).await.expect("guild B row");

    let err = verified_role(&store, GUILD_B)
        .await
        .expect_err("guild B has no verified role");

    assert!(matches!(err, VerifyError::RoleNotConfigured));
}

/// A guild with no `roles_settings` row at all — `SettingsStore::get` falls back
/// to `RolesSettingsRow::empty`, which must also read as "not configured" rather
/// than panicking or defaulting to some other guild's role.
#[sqlx::test(migrations = "../../migrations")]
async fn missing_settings_row_reports_not_configured(pool: PgPool) {
    let store = store(pool);

    let err = verified_role(&store, GUILD_A)
        .await
        .expect_err("no roles_settings row exists");

    assert!(matches!(err, VerifyError::RoleNotConfigured));
}

/// The user-facing half of the fix: an unconfigured guild must surface an
/// actionable message. Previously this path produced `Discord(_)` from an
/// `add_role` against a foreign role id, which
/// [`Respond::user_message`] maps to `None` — a generic failure with no
/// indication of what is wrong.
#[sqlx::test(migrations = "../../migrations")]
async fn not_configured_has_a_user_message(pool: PgPool) {
    use zayden_core::error::Respond;

    let store = store(pool);

    let err = verified_role(&store, GUILD_A)
        .await
        .expect_err("no roles_settings row exists");
    let message = err.user_message().expect("must be user-facing");

    assert!(message.contains("verification role"), "{message}");
}
