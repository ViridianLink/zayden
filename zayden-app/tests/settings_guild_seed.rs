//! Regression net for [honeypot #3] — the `guilds` parent-row seed.
//!
//! All ten `*_settings` tables declare `guild_id … REFERENCES guilds(id)`
//! (`migrations/0003_settings_split`, `0015_family_guild_scope`,
//! `0022_honeypot`), and **nothing seeds `guilds` when the bot joins** —
//! `Handler::guild_create` does not, and the dashboard's guild list comes from
//! the Discord OAuth API rather than the DB. The row first appears when someone
//! chats (`levels::manager`) or opens a ticket (`ticket::support_guild_manager`).
//!
//! So the failing path is the fresh install: invite the bot, configure a module
//! from the dashboard before anyone has spoken, and the upsert dies on `23503`.
//! `/honeypot set` used to dodge this with its own `INSERT INTO guilds`, which
//! is why honeypot is where the asymmetry was visible — one writer seeded, the
//! other did not.
//!
//! These pin the seed to `SettingsStore`, the one place *both* writers go
//! through, rather than to any single row impl or command.
//!
//! **On the assertions:** these deliberately assert the *observable consequence*
//! of the seed rather than counting `guilds` rows. `update` returning `Ok` is
//! itself the check — without the parent row Postgres rejects the write with
//! `23503`, so the happy path cannot be reached by accident. Asserting the
//! consequence also keeps every statement here on SQL that already exists in
//! `.sqlx`, so `cargo test` still builds under `SQLX_OFFLINE=true` (the gate the
//! CC-10 close-out ran); a `SELECT count(*)` would have added a test-only query
//! to a cache that the release build — `cargo build --bin bot`, `docker/
//! Dockerfile.bot:19-25` — never regenerates.

use sqlx::PgPool;
use tokio::sync::broadcast;
use zayden_app::config::tables::{HoneypotSettingsRow, LfgSettingsRow};
use zayden_app::config::{SettingsRow, SettingsStore};

const GUILD: i64 = 1_234_567_890;
const CHANNEL: i64 = 9_876_543_210;

fn store<Row: SettingsRow>(pool: &PgPool) -> SettingsStore<Row> {
    // The receiver is dropped on purpose: `update` fires its invalidation event
    // with `let _ = send(..)`, so a channel with no subscribers is a supported
    // state and not what these tests are about.
    let (events, _rx) = broadcast::channel(16);
    SettingsStore::new(pool.clone(), events)
}

/// The finding's exact scenario: a guild with no `guilds` row — the state of
/// every fresh install until someone speaks. Fails before the fix with
/// `23503 … violates foreign key constraint "honeypot_settings_guild_id_fkey"`.
#[sqlx::test(migrations = "../migrations")]
async fn update_seeds_the_guilds_row_for_an_unseen_guild(
    pool: PgPool,
) -> sqlx::Result<()> {
    let saved = store::<HoneypotSettingsRow>(&pool)
        .update(GUILD, |row| row.channel_id = Some(CHANNEL))
        .await?;

    assert_eq!(saved.channel_id, Some(CHANNEL));

    // Round-trip past the store's cache — the write really landed in Postgres,
    // rather than the cache masking a failed upsert.
    let row =
        HoneypotSettingsRow::select(&pool, GUILD).await?.expect("settings row");
    assert_eq!(row.channel_id, Some(CHANNEL));

    Ok(())
}

/// The seed belongs to the store, not to `HoneypotSettingsRow` — otherwise the
/// fix is a honeypot patch and the other nine tables stay exposed. A second,
/// unrelated row type going through the same `update` is what proves the
/// placement, and it is the assertion that would fail if someone "fixed" this by
/// putting an `INSERT INTO guilds` back into a single command or server fn.
#[sqlx::test(migrations = "../migrations")]
async fn the_seed_belongs_to_the_store_not_the_row(
    pool: PgPool,
) -> sqlx::Result<()> {
    let saved = store::<LfgSettingsRow>(&pool)
        .update(GUILD, |row| row.lfg_channel_id = Some(CHANNEL))
        .await?;

    assert_eq!(saved.lfg_channel_id, Some(CHANNEL));

    Ok(())
}

/// The seed must be `ON CONFLICT DO NOTHING`, never a plain insert or an upsert.
/// `guilds` is a parent table whose children `CASCADE` on delete, so a seed that
/// replaced the row would silently destroy every settings row for that guild —
/// a far worse bug than the one being fixed. Two stores and three writes against
/// one guild would trip any of those wrong shapes.
#[sqlx::test(migrations = "../migrations")]
async fn repeated_seeds_never_disturb_existing_rows(
    pool: PgPool,
) -> sqlx::Result<()> {
    let honeypot = store::<HoneypotSettingsRow>(&pool);

    honeypot.update(GUILD, |row| row.channel_id = Some(CHANNEL)).await?;
    store::<LfgSettingsRow>(&pool)
        .update(GUILD, |row| row.lfg_channel_id = Some(CHANNEL))
        .await?;
    honeypot.update(GUILD, |row| row.exempt_admins = true).await?;

    // Read from the table, not the cache: a clobbering seed would have cascaded
    // both of these away, and the store's cache would still be serving them.
    let hp = HoneypotSettingsRow::select(&pool, GUILD).await?.expect("honeypot row");
    assert_eq!(hp.channel_id, Some(CHANNEL), "earlier write survived");
    assert!(hp.exempt_admins, "later write applied");

    let lfg = LfgSettingsRow::select(&pool, GUILD).await?.expect("lfg row");
    assert_eq!(lfg.lfg_channel_id, Some(CHANNEL), "other table survived");

    Ok(())
}

/// The already-seeded guild — the common case once anyone has chatted. The
/// redundant seed must be a no-op rather than an error.
#[sqlx::test(migrations = "../migrations")]
async fn an_already_seeded_guild_still_updates(pool: PgPool) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
        GUILD
    )
    .execute(&pool)
    .await?;

    let saved = store::<HoneypotSettingsRow>(&pool)
        .update(GUILD, |row| row.channel_id = Some(CHANNEL))
        .await?;

    assert_eq!(saved.channel_id, Some(CHANNEL));

    Ok(())
}
