use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::PgPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecentHero {
    pub discord_id: i64,
    pub hero_id: u32,
    pub games: i64,
    pub last_played: Timestamp,
}

struct RecentRecord {
    discord_id: i64,
    hero_id: i32,
    games: i64,
    last_played: jiff_sqlx::Timestamp,
}

impl RecentRecord {
    fn into_recent(self) -> Option<RecentHero> {
        Some(RecentHero {
            discord_id: self.discord_id,
            hero_id: u32::try_from(self.hero_id).ok()?,
            games: self.games,
            last_played: self.last_played.to_jiff(),
        })
    }
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn recent_heroes(
    pool: &PgPool,
    patch_guid: &str,
    published_at: Timestamp,
    window_start: Timestamp,
) -> sqlx::Result<Vec<RecentHero>> {
    let records = sqlx::query_as!(
        RecentRecord,
        r#"
        SELECT p.discord_id,
               m.hero_id,
               count(*) AS "games!",
               max(m.start_time) AS "last_played!: jiff_sqlx::Timestamp"
        FROM deadlock_players p
        JOIN deadlock_matches_seen m ON m.account_id = p.account_id
        LEFT JOIN deadlock_user_settings s ON s.discord_id = p.discord_id
        WHERE COALESCE(s.patch_briefing, TRUE)
          AND m.start_time >= $3
          AND m.start_time < $2
          AND NOT EXISTS (
              SELECT 1 FROM deadlock_matches_seen later
              WHERE later.account_id = p.account_id AND later.start_time >= $2
          )
          AND NOT EXISTS (
              SELECT 1 FROM deadlock_briefings_sent b
              WHERE b.patch_guid = $1 AND b.discord_id = p.discord_id
          )
        GROUP BY p.discord_id, m.hero_id
        ORDER BY p.discord_id, m.hero_id
        "#,
        patch_guid,
        published_at.to_sqlx() as jiff_sqlx::Timestamp,
        window_start.to_sqlx() as jiff_sqlx::Timestamp
    )
    .fetch_all(pool)
    .await?;

    Ok(records.into_iter().filter_map(RecentRecord::into_recent).collect())
}

pub async fn claim(
    pool: &PgPool,
    patch_guid: &str,
    discord_id: i64,
) -> sqlx::Result<bool> {
    let claimed = sqlx::query_scalar!(
        r#"
        INSERT INTO deadlock_briefings_sent (patch_guid, discord_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        RETURNING discord_id
        "#,
        patch_guid,
        discord_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(claimed.is_some())
}
