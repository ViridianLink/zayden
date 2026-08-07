//! Regression tests for the weekly higher-or-lower payout query.
//!
//! `HigherLower::cron_job` pays 3/2/1 gems to whatever `winners` returns, then
//! zeroes every `weekly_higher_or_lower_score`. The query therefore always runs
//! against a table where most rows are 0, and in a quiet week *every* row is 0.
//! With no filter, `ORDER BY weekly_higher_or_lower_score DESC LIMIT 3` over an
//! all-zero column is an arbitrary-but-deterministic pick, so the same three
//! non-players were announced as winners (and paid) week after week.

use gambling::HigherLowerManager;
use serenity::all::UserId;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations", fixtures("higher_lower_scores"))]
async fn ranks_players_by_weekly_score(pool: PgPool) -> sqlx::Result<()> {
    let mut conn = pool.acquire().await?;

    let winners = HigherLowerManager::winners(&mut conn).await?;

    assert_eq!(winners, vec![UserId::new(100), UserId::new(200)]);

    Ok(())
}

/// A week nobody played must pay out nothing at all.
#[sqlx::test(migrations = "../../migrations", fixtures("higher_lower_scores"))]
async fn skips_everyone_when_no_one_scored(pool: PgPool) -> sqlx::Result<()> {
    let mut conn = pool.acquire().await?;

    HigherLowerManager::reset(&mut conn).await?;

    let winners = HigherLowerManager::winners(&mut conn).await?;

    assert!(
        winners.is_empty(),
        "an all-zero week must have no winners, got {winners:?}"
    );

    Ok(())
}
