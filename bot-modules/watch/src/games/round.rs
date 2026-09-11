use jiff::{Span, Timestamp};
use jiff_sqlx::Timestamp as SqlxTimestamp;
use serenity::all::{GenericChannelId, GuildId, MessageId, UserId};
use sqlx::PgPool;
use zayden_core::as_i64;

pub const ROUND_TTL_MINUTES: i64 = 5;

#[derive(Debug, Clone)]
pub struct NewRound<'a> {
    pub guild_id: GuildId,
    pub channel_id: GenericChannelId,
    pub started_by: UserId,
    pub started_by_name: &'a str,
    pub game: &'a str,
    pub kind: &'a str,
    pub answer: &'a str,
    pub choices: Option<serde_json::Value>,
}

impl NewRound<'_> {
    #[expect(
        trivial_casts,
        reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
    )]
    pub async fn open(self, pool: &PgPool) -> sqlx::Result<i64> {
        let discord_id = as_i64(self.started_by.get());

        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, $2) \
             ON CONFLICT (id) DO NOTHING",
            discord_id,
            self.started_by_name
        )
        .execute(pool)
        .await?;

        let expires_at = Timestamp::now() + Span::new().minutes(ROUND_TTL_MINUTES);

        sqlx::query_scalar!(
            r#"
            INSERT INTO jellyfin_game_rounds
                (guild_id, channel_id, started_by, game, kind, answer, choices,
                 expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
            as_i64(self.guild_id.get()),
            as_i64(self.channel_id.get()),
            discord_id,
            self.game,
            self.kind,
            self.answer,
            self.choices,
            jiff_sqlx::Timestamp::from(expires_at) as jiff_sqlx::Timestamp,
        )
        .fetch_one(pool)
        .await
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RoundRow {
    pub id: i64,
    pub guild_id: i64,
    pub channel_id: i64,
    pub game: String,
    pub kind: String,
    pub answer: String,
    pub choices: Option<serde_json::Value>,
    pub solved_by: Option<i64>,
    pub expires_at: SqlxTimestamp,
}

impl RoundRow {
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Timestamp::now() >= self.expires_at.to_jiff()
    }

    pub async fn get(pool: &PgPool, id: i64) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"SELECT id, guild_id, channel_id, game, kind, answer, choices, solved_by, expires_at AS "expires_at: SqlxTimestamp" FROM jellyfin_game_rounds WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn set_message(
        pool: &PgPool,
        id: i64,
        message_id: MessageId,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_game_rounds SET message_id = $2 WHERE id = $1",
            id,
            as_i64(message_id.get())
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn claim(
        pool: &PgPool,
        id: i64,
        user_id: UserId,
        username: &str,
    ) -> sqlx::Result<Option<String>> {
        let discord_id = as_i64(user_id.get());

        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, $2) \
             ON CONFLICT (id) DO NOTHING",
            discord_id,
            username
        )
        .execute(pool)
        .await?;

        sqlx::query_scalar!(
            r#"
            UPDATE jellyfin_game_rounds
               SET solved_by = $2, solved_at = now()
             WHERE id = $1 AND solved_at IS NULL AND expires_at > now()
            RETURNING answer
            "#,
            id,
            discord_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn release(pool: &PgPool, id: i64) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_game_rounds SET solved_by = NULL, solved_at = NULL WHERE id = $1",
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
