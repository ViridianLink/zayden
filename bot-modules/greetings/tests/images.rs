//! Database-backed tests for the `greeting_images` list.
//!
//! The invariants worth protecting all live in SQL — the guild-scoped delete,
//! the unique constraint, the row-locked cap — so these need a live Postgres.
//! Each `#[sqlx::test]` gets its own migrated database, so `DATABASE_URL` must
//! point at a server the runner may create databases on. Run them against a
//! throwaway server, never a live one.

use greetings::{GreetingImage, GreetingKind, GreetingsError, GuildId};
use sqlx::PgPool;

/// Pre-seeded with two morning images and one night image.
const GUILD: GuildId = GuildId::new(1);
/// Pre-seeded with one morning image, used to prove cross-guild isolation.
const OTHER_GUILD: GuildId = GuildId::new(2);
/// Deliberately absent from the fixture — no `guilds` row at all.
const UNSEEN_GUILD: GuildId = GuildId::new(3);

/// `list`, with the query error unwrapped. A macro rather than a helper fn
/// because `clippy.toml`'s `allow-expect-in-tests` only covers code inside a
/// `#[test]` item, and a free fn in a test binary is not one.
macro_rules! list {
    ($pool:expr, $guild:expr, $kind:expr) => {
        GreetingImage::list(&$pool, $guild, $kind).await.expect("list failed")
    };
}

#[sqlx::test(migrations = "../../migrations", fixtures("greetings"))]
async fn list_returns_only_the_requested_guild_and_kind(pool: PgPool) {
    let morning = list!(pool, GUILD, GreetingKind::Morning);
    let urls: Vec<_> = morning.iter().map(|i| i.url.as_str()).collect();
    assert_eq!(urls, [
        "https://example.com/sunrise-1.gif",
        "https://example.com/sunrise-2.gif"
    ]);

    let night = list!(pool, GUILD, GreetingKind::Night);
    assert_eq!(night.len(), 1, "the night list is separate from morning");
    let night = night.first().expect("just asserted one row");
    assert_eq!(night.url, "https://example.com/moon-1.gif");

    let other = list!(pool, OTHER_GUILD, GreetingKind::Morning);
    assert_eq!(other.len(), 1, "another guild's list must not leak in");
    let other = other.first().expect("just asserted one row");
    assert_eq!(other.url, "https://example.com/other-guild.gif");

    let empty = list!(pool, OTHER_GUILD, GreetingKind::Night);
    assert!(empty.is_empty(), "an unset list is empty, not an error");
}

/// `add` seeds `guilds` itself, so a server that has never had any other
/// setting written still accepts its first image instead of failing with a
/// foreign-key violation.
#[sqlx::test(migrations = "../../migrations", fixtures("greetings"))]
async fn add_seeds_a_guild_that_does_not_exist_yet(pool: PgPool) {
    let row = GreetingImage::add(
        &pool,
        UNSEEN_GUILD,
        GreetingKind::Morning,
        "https://example.com/new.gif",
    )
    .await
    .expect("add must create the missing guilds row");

    assert_eq!(row.url, "https://example.com/new.gif");
    assert_eq!(row.kind, "morning");

    let images = list!(pool, UNSEEN_GUILD, GreetingKind::Morning);
    assert_eq!(images.len(), 1);
}

#[sqlx::test(migrations = "../../migrations", fixtures("greetings"))]
async fn add_rejects_an_invalid_url_before_touching_the_database(pool: PgPool) {
    let err = GreetingImage::add(
        &pool,
        GUILD,
        GreetingKind::Morning,
        "javascript:alert(1)",
    )
    .await
    .expect_err("only https links may be stored");
    assert!(matches!(err, GreetingsError::InvalidUrl(_)), "{err:?}");

    let images = list!(pool, GUILD, GreetingKind::Morning);
    assert_eq!(images.len(), 2, "the rejected add must not have inserted");
}

/// The unique constraint is on `(guild_id, kind, url)`, so the same link may
/// appear in both the morning and night lists, and in another guild, but not
/// twice in one list.
#[sqlx::test(migrations = "../../migrations", fixtures("greetings"))]
async fn duplicates_are_rejected_per_guild_and_kind(pool: PgPool) {
    let url = "https://example.com/sunrise-1.gif";

    let err = GreetingImage::add(&pool, GUILD, GreetingKind::Morning, url)
        .await
        .expect_err("already in this guild's morning list");
    assert!(matches!(err, GreetingsError::DuplicateImage), "{err:?}");

    GreetingImage::add(&pool, GUILD, GreetingKind::Night, url)
        .await
        .expect("the same link in the other list is a different row");

    GreetingImage::add(&pool, OTHER_GUILD, GreetingKind::Morning, url)
        .await
        .expect("another guild's list is independent");
}

/// A failed duplicate add rolls its transaction back, so the `guilds` seed and
/// the count check it performed leave nothing behind.
#[sqlx::test(migrations = "../../migrations", fixtures("greetings"))]
async fn a_rejected_add_leaves_the_list_unchanged(pool: PgPool) {
    let before = list!(pool, GUILD, GreetingKind::Morning);

    let _err = GreetingImage::add(
        &pool,
        GUILD,
        GreetingKind::Morning,
        "https://example.com/sunrise-1.gif",
    )
    .await
    .expect_err("duplicate");

    let after = list!(pool, GUILD, GreetingKind::Morning);
    assert_eq!(before.len(), after.len());
}

#[sqlx::test(migrations = "../../migrations", fixtures("greetings"))]
async fn the_cap_is_per_guild_and_per_kind(pool: PgPool) {
    let max = GreetingImage::MAX_PER_KIND;

    // Two are already seeded, so fill the rest of the morning list.
    for i in 0..(max - 2) {
        GreetingImage::add(
            &pool,
            GUILD,
            GreetingKind::Morning,
            &format!("https://example.com/fill-{i}.gif"),
        )
        .await
        .expect("under the cap");
    }

    let images = list!(pool, GUILD, GreetingKind::Morning);
    assert_eq!(i64::try_from(images.len()).unwrap_or(i64::MAX), max);

    let err = GreetingImage::add(
        &pool,
        GUILD,
        GreetingKind::Morning,
        "https://example.com/one-too-many.gif",
    )
    .await
    .expect_err("the cap is reached");
    let GreetingsError::TooManyImages(reported) = err else {
        panic!("expected TooManyImages, got {err:?}");
    };
    assert_eq!(reported, max, "the error reports the limit it enforced");

    // A full morning list must not block the night list or another guild.
    GreetingImage::add(
        &pool,
        GUILD,
        GreetingKind::Night,
        "https://example.com/night-still-open.gif",
    )
    .await
    .expect("the night list has its own budget");

    GreetingImage::add(
        &pool,
        OTHER_GUILD,
        GreetingKind::Morning,
        "https://example.com/other-still-open.gif",
    )
    .await
    .expect("another guild has its own budget");
}

#[sqlx::test(migrations = "../../migrations", fixtures("greetings"))]
async fn remove_deletes_one_row_and_reports_it(pool: PgPool) {
    let images = list!(pool, GUILD, GreetingKind::Morning);
    let target = images.first().expect("fixture seeds two");

    let removed =
        GreetingImage::remove(&pool, GUILD, target.id).await.expect("remove failed");
    assert!(removed, "an existing row reports true");

    let after = list!(pool, GUILD, GreetingKind::Morning);
    assert_eq!(after.len(), 1);
    assert!(after.iter().all(|i| i.id != target.id));

    let again =
        GreetingImage::remove(&pool, GUILD, target.id).await.expect("remove failed");
    assert!(!again, "removing it twice reports false rather than erroring");
}

/// `remove` is scoped by `guild_id` as well as `id`. Without that, an admin of
/// any guild could delete another guild's images by guessing sequential ids.
#[sqlx::test(migrations = "../../migrations", fixtures("greetings"))]
async fn remove_cannot_reach_another_guilds_row(pool: PgPool) {
    let victim = list!(pool, OTHER_GUILD, GreetingKind::Morning);
    let victim = victim.first().expect("fixture seeds one");

    let removed =
        GreetingImage::remove(&pool, GUILD, victim.id).await.expect("remove failed");
    assert!(!removed, "the id belongs to another guild");

    let after = list!(pool, OTHER_GUILD, GreetingKind::Morning);
    assert_eq!(after.len(), 1, "the other guild's image survives");
}

/// `ON DELETE CASCADE` means removing Zayden from a server takes its curated
/// lists with it rather than orphaning rows.
#[sqlx::test(migrations = "../../migrations", fixtures("greetings"))]
async fn images_cascade_when_the_guild_is_deleted(pool: PgPool) {
    sqlx::query("DELETE FROM guilds WHERE id = $1")
        .bind(i64::from(GUILD))
        .execute(&pool)
        .await
        .expect("delete guild");

    let morning = list!(pool, GUILD, GreetingKind::Morning);
    let night = list!(pool, GUILD, GreetingKind::Night);
    assert!(morning.is_empty() && night.is_empty());

    let other = list!(pool, OTHER_GUILD, GreetingKind::Morning);
    assert_eq!(other.len(), 1, "only the deleted guild's rows go");
}
