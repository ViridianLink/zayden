use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PolledPlayer {
    pub account_id: u32,
    pub discord_id: i64,
    pub seeded_at: Timestamp,
    pub streak: i32,
}

struct PlayerRecord {
    account_id: i64,
    discord_id: i64,
    seeded_at: jiff_sqlx::Timestamp,
    streak: i32,
}

impl PlayerRecord {
    fn into_player(self) -> Option<PolledPlayer> {
        Some(PolledPlayer {
            account_id: u32::try_from(self.account_id).ok()?,
            discord_id: self.discord_id,
            seeded_at: self.seeded_at.to_jiff(),
            streak: self.streak,
        })
    }
}

pub async fn sync(pool: &PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"
        DELETE FROM deadlock_players p
        USING account_links l
        WHERE l.discord_id = p.discord_id
          AND l.provider = 'steam'
          AND CASE
                  WHEN l.external_id ~ '^[0-9]{17}$'
                  THEN l.external_id::bigint - 76561197960265728
              END <> p.account_id
        "#
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO deadlock_players (account_id, discord_id)
        SELECT external_id::bigint - 76561197960265728, discord_id
        FROM account_links
        WHERE provider = 'steam' AND external_id ~ '^[0-9]{17}$'
        ON CONFLICT DO NOTHING
        "#
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await
}

pub async fn all(pool: &PgPool) -> sqlx::Result<Vec<PolledPlayer>> {
    let records = sqlx::query_as!(
        PlayerRecord,
        r#"
        SELECT account_id,
               discord_id,
               seeded_at AS "seeded_at: jiff_sqlx::Timestamp",
               streak
        FROM deadlock_players
        ORDER BY account_id
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(records.into_iter().filter_map(PlayerRecord::into_player).collect())
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn mark_polled(
    pool: &PgPool,
    account_ids: &[i64],
    at: Timestamp,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE deadlock_players SET last_polled_at = $2 WHERE account_id = ANY($1)",
        account_ids,
        at.to_sqlx() as jiff_sqlx::Timestamp
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_streak(
    pool: &PgPool,
    account_id: u32,
    streak: i32,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE deadlock_players SET streak = $2 WHERE account_id = $1",
        i64::from(account_id),
        streak
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn is_linked(pool: &PgPool, discord_id: i64) -> sqlx::Result<bool> {
    let linked = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM account_links
            WHERE discord_id = $1 AND provider = 'steam'
        ) AS "linked!"
        "#,
        discord_id
    )
    .fetch_one(pool)
    .await?;

    Ok(linked)
}
