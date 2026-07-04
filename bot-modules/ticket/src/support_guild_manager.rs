use async_trait::async_trait;
use serenity::all::{ChannelId, GuildId, RoleId};
use sqlx::{Database, FromRow, PgPool, Pool, Postgres};
use zayden_core::{as_i64, as_u64};

#[async_trait]
pub trait TicketGuildManager<Db: Database> {
    async fn get(
        pool: &Pool<Db>,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Option<TicketGuildRow>>;

    async fn update_thread_id(
        pool: &Pool<Db>,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<()>;
}

#[derive(FromRow)]
pub struct TicketGuildRow {
    pub id: i64,
    pub thread_id: i32,
    pub support_channel_id: Option<i64>,
    pub support_role_ids: Vec<i64>,
    pub faq_channel_id: Option<i64>,
}

impl TicketGuildRow {
    #[must_use]
    pub fn channel_id(&self) -> Option<ChannelId> {
        self.support_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }

    #[must_use]
    pub fn role_ids(&self) -> Vec<RoleId> {
        self.support_role_ids.iter().map(|&id| RoleId::new(as_u64(id))).collect()
    }

    #[must_use]
    pub fn faq_channel_id(&self) -> Option<ChannelId> {
        self.faq_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }
}

pub struct GuildTable;

#[async_trait]
impl TicketGuildManager<Postgres> for GuildTable {
    async fn get(
        pool: &PgPool,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Option<TicketGuildRow>> {
        let id = id.into();

        let Some(row) = sqlx::query!(
            r#"
            SELECT
                s.guild_id AS id,
                COALESCE(t.thread_id, 0) AS "thread_id!",
                s.support_channel_id,
                s.faq_channel_id
            FROM support_settings s
            LEFT JOIN ticket_settings t ON t.guild_id = s.guild_id
            WHERE s.guild_id = $1
            "#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await?
        else {
            return Ok(None);
        };

        let support_role_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT role_id FROM guild_support_roles WHERE guild_id = $1",
            as_i64(id.get())
        )
        .fetch_all(pool)
        .await?;

        Ok(Some(TicketGuildRow {
            id: row.id,
            thread_id: row.thread_id,
            support_channel_id: row.support_channel_id,
            support_role_ids,
            faq_channel_id: row.faq_channel_id,
        }))
    }

    async fn update_thread_id(
        pool: &PgPool,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO ticket_settings (guild_id, thread_id)
            VALUES ($1, 1)
            ON CONFLICT (guild_id) DO UPDATE SET
                thread_id = ticket_settings.thread_id + 1,
                updated_at = now()
            "#,
            as_i64(id.into().get())
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
