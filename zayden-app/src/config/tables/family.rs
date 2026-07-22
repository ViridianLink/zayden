use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FamilySettingsRow {
    pub guild_id: i64,
    pub max_partners: i32,
}

impl SettingsRow for FamilySettingsRow {
    const TABLE: &'static str = "family_settings";

    fn empty(guild_id: i64) -> Self {
        Self { guild_id, max_partners: 1 }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, max_partners
            FROM family_settings
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
            INSERT INTO family_settings (guild_id, max_partners)
            VALUES ($1, $2)
            ON CONFLICT (guild_id) DO UPDATE SET
                max_partners = EXCLUDED.max_partners,
                updated_at = now()
            RETURNING guild_id, max_partners
            "#,
            self.guild_id,
            self.max_partners
        )
        .fetch_one(pool)
        .await
    }
}
