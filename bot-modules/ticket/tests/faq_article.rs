//! Storage and retrieval of the FAQ articles written from solved tickets.
//!
//! Two guards here are the ones that would be expensive to discover in
//! production:
//!
//! - `get` is guild scoped. The article id travels through a select menu's
//!   `custom_id`, which a user can forge, so an unscoped lookup would serve one
//!   guild's article to another. Dropping `guild_id` from the `WHERE` clause fails
//!   `an_article_is_invisible_to_another_guild`.
//! - One article per source thread. A double invocation of `/ticket solved` must not
//!   publish the same article twice. Dropping `faq_articles_thread_idx` fails
//!   `a_second_insert_for_the_same_thread_is_refused`.

use sqlx::PgPool;
use ticket::{FaqArticle, NewArticle};

const GUILD: i64 = 1;
const OTHER_GUILD: i64 = 2;
const THREAD: i64 = 500;

/// Kept in step with `DUPLICATE_RANK` in `faq/generate.rs`.
const DUPLICATE_RANK: f32 = 0.9;

const fn article<'a>(
    title: &'a str,
    content: &'a str,
    tags: &'a [String],
) -> NewArticle<'a> {
    NewArticle {
        title,
        summary: "One sentence of summary.",
        content,
        category: Some("radarr"),
        tags,
    }
}

#[sqlx::test(migrations = "../../migrations", fixtures("faq_articles"))]
async fn an_article_round_trips(pool: PgPool) -> sqlx::Result<()> {
    let tags = vec![String::from("radarr"), String::from("proxy")];

    let stored =
        FaqArticle::create(&pool, GUILD, article("Radarr 502", "body", &tags))
            .await?;

    let read =
        FaqArticle::get(&pool, GUILD, stored.id).await?.expect("just created");

    assert_eq!(read.title, "Radarr 502");
    assert_eq!(read.content, "body");
    assert_eq!(read.tags, tags);
    assert!(!read.generated);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("faq_articles"))]
async fn an_article_is_invisible_to_another_guild(pool: PgPool) -> sqlx::Result<()> {
    let stored =
        FaqArticle::create(&pool, GUILD, article("Radarr 502", "body", &[])).await?;

    assert!(FaqArticle::get(&pool, OTHER_GUILD, stored.id).await?.is_none());
    assert!(!FaqArticle::delete(&pool, OTHER_GUILD, stored.id).await?);
    assert!(FaqArticle::get(&pool, GUILD, stored.id).await?.is_some());

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("faq_articles"))]
async fn search_finds_an_article_by_its_body(pool: PgPool) -> sqlx::Result<()> {
    FaqArticle::create(
        &pool,
        GUILD,
        article("Radarr 502", "Restart the reverse proxy container", &[]),
    )
    .await?;

    FaqArticle::create(&pool, GUILD, article("Sonarr indexers", "Unrelated", &[]))
        .await?;

    let hits = FaqArticle::search(&pool, GUILD, "reverse proxy", 5).await?;

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Radarr 502");

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("faq_articles"))]
async fn search_takes_arbitrary_user_input(pool: PgPool) -> sqlx::Result<()> {
    // `to_tsquery` would error on this; `websearch_to_tsquery` must not.
    let hits =
        FaqArticle::search(&pool, GUILD, "why is my & | ! radarr :", 5).await?;

    assert!(hits.is_empty());

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("faq_articles"))]
async fn search_is_guild_scoped(pool: PgPool) -> sqlx::Result<()> {
    FaqArticle::create(
        &pool,
        GUILD,
        article("Radarr 502", "Restart the reverse proxy container", &[]),
    )
    .await?;

    assert!(
        FaqArticle::search(&pool, OTHER_GUILD, "reverse proxy", 5).await?.is_empty()
    );

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("faq_articles"))]
async fn a_second_insert_for_the_same_thread_is_refused(
    pool: PgPool,
) -> sqlx::Result<()> {
    let first = FaqArticle::insert_generated(
        &pool,
        GUILD,
        THREAD,
        article("First", "a", &[]),
    )
    .await?;

    assert!(first.is_some());

    let second = FaqArticle::insert_generated(
        &pool,
        GUILD,
        THREAD,
        article("Second", "b", &[]),
    )
    .await?;

    assert!(second.is_none());
    assert_eq!(FaqArticle::list(&pool, GUILD, 25).await?.len(), 1);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("faq_articles"))]
async fn a_generated_article_records_its_thread(pool: PgPool) -> sqlx::Result<()> {
    let stored = FaqArticle::insert_generated(
        &pool,
        GUILD,
        THREAD,
        article("First", "a", &[]),
    )
    .await?
    .expect("the thread has no article yet");

    assert_eq!(stored.source_thread_id, Some(THREAD));
    assert!(stored.generated);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("faq_articles"))]
async fn a_restatement_outranks_a_title_that_merely_overlaps(
    pool: PgPool,
) -> sqlx::Result<()> {
    FaqArticle::create(
        &pool,
        GUILD,
        article(
            "Fixing Radarr error 502 bad gateway",
            "Restart the reverse proxy container, then check the Radarr logs \
             for the bad gateway error.",
            &[],
        ),
    )
    .await?;

    let rank = async |title| FaqArticle::best_match_rank(&pool, GUILD, title).await;

    // A different topic shares no full term set, so it never reaches the
    // threshold comparison at all.
    assert_eq!(rank("Radarr not importing downloads").await?, None);
    assert_eq!(rank("Configuring Plex remote access").await?, None);

    // A restatement of the same article saturates ts_rank.
    let restatement = rank("Radarr error 502 bad gateway").await?;

    assert!(
        restatement.is_some_and(|rank| rank >= DUPLICATE_RANK),
        "a restatement scored {restatement:?}, below the duplicate threshold"
    );

    // Sharing vocabulary is not the same as restating. These must stay clear of
    // the threshold or genuinely new articles get dropped.
    for title in ["Radarr container restart", "reverse proxy", "Radarr logs"] {
        let overlap = rank(title).await?;

        assert!(
            overlap.is_none_or(|rank| rank < DUPLICATE_RANK),
            "{title:?} scored {overlap:?}, at or above the duplicate threshold"
        );
    }

    Ok(())
}

#[sqlx::test(migrations = "../../migrations", fixtures("faq_articles"))]
async fn an_update_replaces_the_body(pool: PgPool) -> sqlx::Result<()> {
    let stored =
        FaqArticle::create(&pool, GUILD, article("Radarr 502", "old", &[])).await?;

    let updated = FaqArticle::update(
        &pool,
        GUILD,
        stored.id,
        article("Radarr 502", "new", &[]),
    )
    .await?
    .expect("the article belongs to this guild");

    assert_eq!(updated.content, "new");

    assert!(
        FaqArticle::update(
            &pool,
            OTHER_GUILD,
            stored.id,
            article("Radarr 502", "hijacked", &[])
        )
        .await?
        .is_none()
    );

    Ok(())
}
