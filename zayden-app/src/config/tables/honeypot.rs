use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HoneypotSettingsRow {
    pub guild_id: i64,
    pub channel_id: Option<i64>,
    pub exempt_admins: bool,
    pub exempt_role_id: Option<i64>,
}

impl SettingsRow for HoneypotSettingsRow {
    const TABLE: &'static str = "honeypot_settings";

    fn empty(guild_id: i64) -> Self {
        Self {
            guild_id,
            channel_id: None,
            exempt_admins: false,
            exempt_role_id: None,
        }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, channel_id, exempt_admins, exempt_role_id
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
                (guild_id, channel_id, exempt_admins, exempt_role_id)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (guild_id) DO UPDATE SET
                channel_id = EXCLUDED.channel_id,
                exempt_admins = EXCLUDED.exempt_admins,
                exempt_role_id = EXCLUDED.exempt_role_id,
                updated_at = now()
            RETURNING guild_id, channel_id, exempt_admins, exempt_role_id
            "#,
            self.guild_id,
            self.channel_id,
            self.exempt_admins,
            self.exempt_role_id
        )
        .fetch_one(pool)
        .await
    }
}
