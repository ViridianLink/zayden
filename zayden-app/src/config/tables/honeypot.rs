use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HoneypotSettingsRow {
    pub guild_id: i64,
    pub channel_id: Option<i64>,
    pub exempt_admins: bool,
    pub exempt_role_id: Option<i64>,
    pub purge_seconds: i32,
}

impl HoneypotSettingsRow {
    pub const DEFAULT_PURGE_SECONDS: i32 = 24 * 60 * 60;
    pub const MAX_PURGE_SECONDS: i32 = 7 * 24 * 60 * 60;

    #[must_use]
    pub fn parse_purge_seconds(input: &str) -> i32 {
        input
            .trim()
            .parse::<i32>()
            .unwrap_or(Self::DEFAULT_PURGE_SECONDS)
            .clamp(0, Self::MAX_PURGE_SECONDS)
    }

    #[must_use]
    pub fn purge_seconds_u32(&self) -> u32 {
        u32::try_from(self.purge_seconds.clamp(0, Self::MAX_PURGE_SECONDS))
            .unwrap_or(0)
    }
}

impl SettingsRow for HoneypotSettingsRow {
    const TABLE: &'static str = "honeypot_settings";

    fn empty(guild_id: i64) -> Self {
        Self {
            guild_id,
            channel_id: None,
            exempt_admins: false,
            exempt_role_id: None,
            purge_seconds: Self::DEFAULT_PURGE_SECONDS,
        }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, channel_id, exempt_admins, exempt_role_id,
                   purge_seconds
            FROM honeypot_settings
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
            INSERT INTO honeypot_settings
                (guild_id, channel_id, exempt_admins, exempt_role_id,
                 purge_seconds)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (guild_id) DO UPDATE SET
                channel_id = EXCLUDED.channel_id,
                exempt_admins = EXCLUDED.exempt_admins,
                exempt_role_id = EXCLUDED.exempt_role_id,
                purge_seconds = EXCLUDED.purge_seconds,
                updated_at = now()
            RETURNING guild_id, channel_id, exempt_admins, exempt_role_id,
                      purge_seconds
            "#,
            self.guild_id,
            self.channel_id,
            self.exempt_admins,
            self.exempt_role_id,
            self.purge_seconds
        )
        .fetch_one(pool)
        .await
    }
}
