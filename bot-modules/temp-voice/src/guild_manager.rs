use serenity::all::{ChannelId, GuildId};
use sqlx::postgres::PgQueryResult;
use sqlx::{FromRow, PgPool};
use zayden_core::{as_i64, as_u64};

#[derive(FromRow)]
pub struct TempVoiceRow {
    pub id: i64,
    pub temp_voice_category: Option<i64>,
    pub temp_voice_creator_channel: Option<i64>,
}

impl TempVoiceRow {
    #[must_use]
    pub fn guild_id(&self) -> GuildId {
        GuildId::from(as_u64(self.id))
    }

    #[must_use]
    pub fn category(&self) -> Option<ChannelId> {
        self.temp_voice_category.map(|id| ChannelId::from(as_u64(id)))
    }

    #[must_use]
    pub fn creator_channel(&self) -> Option<ChannelId> {
        self.temp_voice_creator_channel.map(|id| ChannelId::from(as_u64(id)))
    }

    pub async fn save(
        pool: &PgPool,
        id: GuildId,
        category: ChannelId,
        creator_channel: ChannelId,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            r#"
            INSERT INTO temp_voice_settings (guild_id, temp_voice_category, temp_voice_creator_channel)
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id) DO UPDATE SET
                temp_voice_category = EXCLUDED.temp_voice_category,
                temp_voice_creator_channel = EXCLUDED.temp_voice_creator_channel,
                updated_at = now()
            "#,
            as_i64(id.get()),
            as_i64(category.get()),
            as_i64(creator_channel.get())
        )
        .execute(pool)
        .await
    }

    pub async fn get(pool: &PgPool, id: GuildId) -> sqlx::Result<Self> {
        sqlx::query_as!(
            TempVoiceRow,
            r#"
            SELECT guild_id AS id, temp_voice_category, temp_voice_creator_channel
            FROM temp_voice_settings
            WHERE guild_id = $1
            "#,
            as_i64(id.get())
        )
        .fetch_one(pool)
        .await
    }

    pub async fn get_category(
        pool: &PgPool,
        id: GuildId,
    ) -> sqlx::Result<ChannelId> {
        let category = sqlx::query_scalar!(
            "SELECT temp_voice_category FROM temp_voice_settings WHERE guild_id = $1",
            as_i64(id.get())
        )
        .fetch_one(pool)
        .await?;

        Ok(ChannelId::new(as_u64(category.ok_or(sqlx::Error::RowNotFound)?)))
    }

    pub async fn get_creator_channel(
        pool: &PgPool,
        id: GuildId,
    ) -> sqlx::Result<Option<ChannelId>> {
        let creator_channel = sqlx::query_scalar!(
            "SELECT temp_voice_creator_channel FROM temp_voice_settings WHERE guild_id = $1",
            as_i64(id.get())
        )
        .fetch_one(pool)
        .await?;

        Ok(creator_channel.map(|id| ChannelId::new(as_u64(id))))
    }
}
