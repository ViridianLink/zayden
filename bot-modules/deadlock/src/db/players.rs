use deadlock_api::rank::Badge;
use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::PgPool;

use crate::squad::LeaderboardEntry;

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

#[derive(Debug, Clone, Copy)]
pub struct SquadPlayer {
    pub account_id: u32,
    pub discord_id: i64,
    pub badge: Option<i32>,
    pub streak: i32,
    pub announced_streak: i32,
}

struct SquadRecord {
    account_id: i64,
    discord_id: i64,
    badge: Option<i32>,
    streak: i32,
    announced_streak: i32,
}

impl SquadRecord {
    fn into_player(self) -> Option<SquadPlayer> {
        Some(SquadPlayer {
            account_id: u32::try_from(self.account_id).ok()?,
            discord_id: self.discord_id,
            badge: self.badge,
            streak: self.streak,
            announced_streak: self.announced_streak,
        })
    }
}

struct MemberRecord {
    discord_id: i64,
    badge: Option<i32>,
    streak: i32,
}

impl MemberRecord {
    fn into_entry(self) -> LeaderboardEntry {
        LeaderboardEntry {
            discord_id: self.discord_id,
            badge: self
                .badge
                .and_then(|raw| u16::try_from(raw).ok())
                .and_then(Badge::from_raw),
            streak: self.streak,
        }
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

pub async fn squad_players(pool: &PgPool) -> sqlx::Result<Vec<SquadPlayer>> {
    let records = sqlx::query_as!(
        SquadRecord,
        r#"
        SELECT p.account_id,
               p.discord_id,
               p.badge,
               p.streak,
               p.announced_streak
        FROM deadlock_players p
        WHERE EXISTS (
            SELECT 1 FROM deadlock_guild_members m
            WHERE m.discord_id = p.discord_id AND m.squad
        )
        ORDER BY p.account_id
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(records.into_iter().filter_map(SquadRecord::into_player).collect())
}

pub async fn set_badge(
    pool: &PgPool,
    account_id: u32,
    badge: i32,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE deadlock_players SET badge = $2 WHERE account_id = $1",
        i64::from(account_id),
        badge
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_announced_streak(
    pool: &PgPool,
    account_id: u32,
    announced: i32,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE deadlock_players SET announced_streak = $2 WHERE account_id = $1",
        i64::from(account_id),
        announced
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn guild_squad(
    pool: &PgPool,
    guild_id: i64,
) -> sqlx::Result<Vec<LeaderboardEntry>> {
    let records = sqlx::query_as!(
        MemberRecord,
        r#"
        SELECT p.discord_id, p.badge, p.streak
        FROM deadlock_guild_members m
        JOIN deadlock_players p ON p.discord_id = m.discord_id
        WHERE m.guild_id = $1 AND m.squad
        ORDER BY p.discord_id
        "#,
        guild_id
    )
    .fetch_all(pool)
    .await?;

    Ok(records.into_iter().map(MemberRecord::into_entry).collect())
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
