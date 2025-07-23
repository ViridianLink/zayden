mod commands;
pub use commands::Voice;

pub mod events;

use async_trait::async_trait;
use serenity::all::{ChannelId, Context, CreateCommand, GuildId, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use temp_voice::voice_channel_manager::VoiceChannelMode;
use temp_voice::{TempVoiceGuildManager, TempVoiceRow, VoiceChannelManager, VoiceChannelRow};
use zayden_core::SlashCommand;

use crate::sqlx_lib::GuildTable;

pub fn register(ctx: &Context) -> CreateCommand<'_> {
    Voice::register(ctx).unwrap()
}

#[async_trait]
impl TempVoiceGuildManager<Postgres> for GuildTable {
    async fn save(
        pool: &PgPool,
        id: GuildId,
        category: ChannelId,
        creator_channel: ChannelId,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            r#"
            INSERT INTO guilds (id, temp_voice_category, temp_voice_creator_channel)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE
            SET temp_voice_category = $2, temp_voice_creator_channel = $3
            "#,
            id.get() as i64,
            category.get() as i64,
            creator_channel.get() as i64
        )
        .execute(pool)
        .await
    }

    async fn get(pool: &PgPool, id: GuildId) -> sqlx::Result<TempVoiceRow> {
        sqlx::query_as!(
            TempVoiceRow,
            r#"SELECT id, temp_voice_category, temp_voice_creator_channel FROM guilds WHERE id = $1"#,
            id.get() as i64
        )
        .fetch_one(pool)
        .await
    }

    async fn get_category(pool: &PgPool, id: GuildId) -> sqlx::Result<ChannelId> {
        let category = sqlx::query_scalar!(
            r#"SELECT temp_voice_category FROM guilds WHERE id = $1"#,
            id.get() as i64
        )
        .fetch_one(pool)
        .await?;

        Ok(ChannelId::new(category.unwrap() as u64))
    }

    async fn get_creator_channel(pool: &PgPool, id: GuildId) -> sqlx::Result<Option<ChannelId>> {
        let row = sqlx::query!(
            r#"SELECT temp_voice_creator_channel FROM guilds WHERE id = $1"#,
            id.get() as i64
        )
        .fetch_one(pool)
        .await?;

        let channel_id = row
            .temp_voice_creator_channel
            .map(|id| ChannelId::new(id as u64));

        Ok(channel_id)
    }
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "temp_voice_mode")]
struct TempVoiceMode(VoiceChannelMode);

impl From<VoiceChannelMode> for TempVoiceMode {
    fn from(mode: VoiceChannelMode) -> Self {
        TempVoiceMode(mode)
    }
}

impl From<TempVoiceMode> for VoiceChannelMode {
    fn from(wrapper: TempVoiceMode) -> Self {
        wrapper.0
    }
}

struct VoiceChannelTable;

#[async_trait]
impl VoiceChannelManager<Postgres> for VoiceChannelTable {
    async fn get(pool: &PgPool, id: ChannelId) -> sqlx::Result<Option<VoiceChannelRow>> {
        let row = sqlx::query_as!(
            VoiceChannelRow,
            r#"SELECT id, owner_id, trusted_ids, invites, password, persistent, mode AS "mode: TempVoiceMode" FROM voice_channels WHERE id = $1"#,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    async fn count_persistent_channels(pool: &PgPool, user_id: UserId) -> sqlx::Result<i64> {
        let count = sqlx::query!(
            r#"SELECT COUNT(*) FROM voice_channels WHERE owner_id = $1 AND persistent = true"#,
            user_id.get() as i64
        )
        .fetch_one(pool)
        .await?
        .count;

        Ok(count.unwrap())
    }

    async fn save(pool: &PgPool, row: VoiceChannelRow) -> sqlx::Result<PgQueryResult> {
        let mode = TempVoiceMode::from(row.mode);

        sqlx::query!(
            r#"
            INSERT INTO voice_channels (id, owner_id, trusted_ids, password, persistent, invites, mode)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE
            SET owner_id = $2, trusted_ids = $3, password = $4, persistent = $5, invites = $6, mode = $7
            "#,
            row.id,
            row.owner_id,
            &row.trusted_ids,
            row.password,
            row.persistent,
            &row.invites,
            mode as TempVoiceMode
        )
        .execute(pool)
        .await
    }

    async fn delete(pool: &PgPool, id: ChannelId) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            r#"DELETE FROM voice_channels WHERE id = $1"#,
            id.get() as i64
        )
        .execute(pool)
        .await
    }
}
