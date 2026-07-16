use serenity::all::{ChannelId, GuildId, RoleId};
use sqlx::PgPool;
use zayden_app::config::{SettingsStore, SupportSettingsRow, TicketSettingsRow};
use zayden_core::{as_i64, as_u64};

#[derive(Clone, Copy)]
pub struct TicketStores<'a> {
    pub support: &'a SettingsStore<SupportSettingsRow>,
    pub ticket: &'a SettingsStore<TicketSettingsRow>,
}

#[derive(Debug)]
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

    pub async fn get(
        stores: TicketStores<'_>,
        pool: &PgPool,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Option<Self>> {
        let id = as_i64(id.into().get());

        let Some(support) = stores.support.try_get(id).await? else {
            return Ok(None);
        };

        let thread_id = stores.ticket.get(id).await?.thread_id;

        let support_role_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT role_id FROM guild_support_roles WHERE guild_id = $1",
            id
        )
        .fetch_all(pool)
        .await?;

        Ok(Some(Self {
            id,
            thread_id,
            support_channel_id: support.support_channel_id,
            support_role_ids,
            faq_channel_id: support.faq_channel_id,
        }))
    }

    pub async fn increment_thread_id(
        store: &SettingsStore<TicketSettingsRow>,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<()> {
        store.update(as_i64(id.into().get()), |row| row.thread_id += 1).await?;

        Ok(())
    }
}
