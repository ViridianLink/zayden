use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JellyfinSettingsRow {
    pub guild_id: i64,
    pub party_channel_id: Option<i64>,
    pub game_channel_id: Option<i64>,
    pub guests_enabled: bool,
    pub max_concurrent_guests: i32,
}

impl JellyfinSettingsRow {
    pub const DEFAULT_GUESTS_ENABLED: bool = false;
    pub const DEFAULT_MAX_GUESTS: i32 = 20;
    pub const MAX_MAX_GUESTS: i32 = 100;

    #[must_use]
    pub fn clamped_max_guests(&self) -> i32 {
        self.max_concurrent_guests.clamp(0, Self::MAX_MAX_GUESTS)
    }
}

impl SettingsRow for JellyfinSettingsRow {
    const TABLE: &'static str = "jellyfin_settings";

    fn empty(guild_id: i64) -> Self {
        Self {
            guild_id,
            party_channel_id: None,
            game_channel_id: None,
            guests_enabled: Self::DEFAULT_GUESTS_ENABLED,
            max_concurrent_guests: Self::DEFAULT_MAX_GUESTS,
        }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, party_channel_id, game_channel_id, guests_enabled,
                   max_concurrent_guests
            FROM jellyfin_settings
            WHERE guild_id = $1
            "#,
            guild_id
        )
        .fetch_optional(pool)
        .await
    }

    async fn upsert(&self, pool: &PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            INSERT INTO jellyfin_settings
                (guild_id, party_channel_id, game_channel_id, guests_enabled,
                 max_concurrent_guests)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (guild_id) DO UPDATE SET
                party_channel_id = EXCLUDED.party_channel_id,
                game_channel_id = EXCLUDED.game_channel_id,
                guests_enabled = EXCLUDED.guests_enabled,
                max_concurrent_guests = EXCLUDED.max_concurrent_guests,
                updated_at = now()
            RETURNING guild_id, party_channel_id, game_channel_id, guests_enabled,
                      max_concurrent_guests
            "#,
            self.guild_id,
            self.party_channel_id,
            self.game_channel_id,
            self.guests_enabled,
            self.max_concurrent_guests
        )
        .fetch_one(pool)
        .await
    }
}
