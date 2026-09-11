use serenity::all::{GuildId, UserId};
use sqlx::PgPool;
use zayden_core::as_i64;

pub const POINTS_CORRECT: i32 = 10;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScoreRow {
    pub user_id: i64,
    pub points: i32,
    pub correct: i32,
    pub played: i32,
}

impl ScoreRow {
    #[must_use]
    pub const fn user(&self) -> UserId {
        UserId::new(self.user_id.cast_unsigned())
    }

    pub async fn record(
        pool: &PgPool,
        guild_id: GuildId,
        user_id: UserId,
        username: &str,
        game: &str,
        correct: bool,
    ) -> sqlx::Result<()> {
        let guild = as_i64(guild_id.get());
        let discord_id = as_i64(user_id.get());

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, $2) \
             ON CONFLICT (id) DO NOTHING",
            discord_id,
            username
        )
        .execute(pool)
        .await?;

        let points = if correct { POINTS_CORRECT } else { 0 };
        let hit = i32::from(correct);

        sqlx::query!(
            r#"
            INSERT INTO jellyfin_game_scores
                (guild_id, user_id, game, points, correct, played, last_played_at)
            VALUES ($1, $2, $3, $4, $5, 1, now())
            ON CONFLICT (guild_id, user_id, game) DO UPDATE SET
                points = jellyfin_game_scores.points + EXCLUDED.points,
                correct = jellyfin_game_scores.correct + EXCLUDED.correct,
                played = jellyfin_game_scores.played + 1,
                last_played_at = now()
            "#,
            guild,
            discord_id,
            game,
            points,
            hit,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn leaderboard(
        pool: &PgPool,
        guild_id: GuildId,
        game: Option<&str>,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT user_id,
                   SUM(points)::int AS "points!",
                   SUM(correct)::int AS "correct!",
                   SUM(played)::int AS "played!"
            FROM jellyfin_game_scores
            WHERE guild_id = $1 AND ($2::text IS NULL OR game = $2)
            GROUP BY user_id
            ORDER BY "points!" DESC
            LIMIT $3
            "#,
            as_i64(guild_id.get()),
            game,
            limit
        )
        .fetch_all(pool)
        .await
    }
}
