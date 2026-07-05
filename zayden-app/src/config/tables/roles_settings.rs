use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RolesSettingsRow {
    pub guild_id: i64,
    pub artist_role_id: Option<i64>,
    pub sleep_role_id: Option<i64>,
}

impl SettingsRow for RolesSettingsRow {
    const TABLE: &'static str = "roles_settings";

    fn empty(guild_id: i64) -> Self {
        Self { guild_id, artist_role_id: None, sleep_role_id: None }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, artist_role_id, sleep_role_id
            FROM roles_settings
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
            INSERT INTO roles_settings (guild_id, artist_role_id, sleep_role_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id) DO UPDATE SET
                artist_role_id = EXCLUDED.artist_role_id,
                sleep_role_id = EXCLUDED.sleep_role_id,
                updated_at = now()
            RETURNING guild_id, artist_role_id, sleep_role_id
            "#,
            self.guild_id,
            self.artist_role_id,
            self.sleep_role_id
        )
        .fetch_one(pool)
        .await
    }
}
