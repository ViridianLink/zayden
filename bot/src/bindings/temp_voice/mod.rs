mod commands;
pub use commands::Voice;

pub mod events;

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(Voice);
}

use async_trait::async_trait;
use serenity::all::{ChannelId, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use temp_voice::voice_channel_manager::VoiceChannelMode;
use temp_voice::{VoiceChannelManager, VoiceChannelRow};
use zayden_core::as_i64;

#[derive(sqlx::Type)]
#[sqlx(type_name = "temp_voice_mode")]
struct TempVoiceMode(VoiceChannelMode);

impl From<VoiceChannelMode> for TempVoiceMode {
    fn from(mode: VoiceChannelMode) -> Self {
        Self(mode)
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
    async fn get(
        pool: &PgPool,
        id: ChannelId,
    ) -> sqlx::Result<Option<VoiceChannelRow>> {
        let row = sqlx::query_as!(
            VoiceChannelRow,
            r#"SELECT 
                vc.id, 
                vc.owner_id, 
                COALESCE(
                    (SELECT array_agg(user_id) FROM voice_channel_trusted_users WHERE channel_id = vc.id), 
                    ARRAY[]::int[]
                ) AS "trusted_ids!", 
                COALESCE(
                    (SELECT array_agg(user_id) FROM voice_channel_invites WHERE channel_id = vc.id), 
                    ARRAY[]::int[]
                ) AS "invites!", 
                vc.password, 
                vc.persistent, 
                vc.mode AS "mode: TempVoiceMode" 
            FROM voice_channels vc
            WHERE vc.id = $1;"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    async fn count_persistent_channels(
        pool: &PgPool,
        user_id: UserId,
    ) -> sqlx::Result<i64> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM voice_channels WHERE owner_id = $1 AND persistent = true"#,
            as_i64(user_id.get())
        )
        .fetch_one(pool)
        .await?;

        Ok(count.ok_or(sqlx::Error::RowNotFound)?)
    }

    #[expect(
        trivial_casts,
        reason = "sqlx requires explicit type for custom temp_voice_mode pgtype"
    )]
    async fn save(
        pool: &PgPool,
        row: VoiceChannelRow,
    ) -> sqlx::Result<PgQueryResult> {
        let mode = TempVoiceMode::from(row.mode);

        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            r#"
            INSERT INTO voice_channels (id, owner_id, password, persistent, mode)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE
            SET owner_id = $2, password = $3, persistent = $4, mode = $5
            "#,
            row.id,
            row.owner_id,
            row.password,
            row.persistent,
            mode as TempVoiceMode
        )
        .execute(&mut *tx)
        .await?;

        let result2 = sqlx::query!(
            r#"
            WITH deleted AS (
                DELETE FROM voice_channel_trusted_users WHERE channel_id = $1
            )
            INSERT INTO voice_channel_trusted_users (channel_id, user_id)
            SELECT $1, * FROM UNNEST($2::bigint[])
            "#,
            row.id,
            &row.trusted_ids
        )
        .execute(&mut *tx)
        .await?;

        let result3 = sqlx::query!(
            r#"
            WITH deleted AS (
                DELETE FROM voice_channel_invites WHERE channel_id = $1
            )
            INSERT INTO voice_channel_invites (channel_id, user_id)
            SELECT $1, * FROM UNNEST($2::bigint[])
            "#,
            row.id,
            &row.invites
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        result.extend([result2, result3]);

        Ok(result)
    }

    async fn delete(pool: &PgPool, id: ChannelId) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(r#"DELETE FROM voice_channels WHERE id = $1"#, as_i64(id.get()))
            .execute(pool)
            .await
    }
}
