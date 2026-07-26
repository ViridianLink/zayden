use futures::TryStreamExt;
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
    pub support_role_ids: Vec<RoleId>,
    pub faq_channel_id: Option<i64>,
}

impl TicketGuildRow {
    #[must_use]
    pub fn channel_id(&self) -> Option<ChannelId> {
        self.support_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }

    #[must_use]
    pub fn role_ids(&self) -> &[RoleId] {
        &self.support_role_ids
    }

    #[must_use]
    pub fn faq_channel_id(&self) -> Option<ChannelId> {
        self.faq_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }

    pub async fn get(
        stores: TicketStores<'_>,
        pool: &PgPool,
        guild_id: GuildId,
    ) -> sqlx::Result<Option<Self>> {
        let id = as_i64(guild_id.get());

        let Some(support) = stores.support.try_get(id).await? else {
            return Ok(None);
        };

        let thread_id = stores.ticket.get(id).await?.thread_id;

        let support_role_ids = SupportRoles::ids(pool, guild_id).await?;

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
        id: GuildId,
    ) -> sqlx::Result<()> {
        store.update(as_i64(id.get()), |row| row.thread_id += 1).await?;

        Ok(())
    }
}

pub struct SupportRoles;

impl SupportRoles {
    pub async fn ids(pool: &PgPool, guild_id: GuildId) -> sqlx::Result<Vec<RoleId>> {
        sqlx::query_scalar!(
            "SELECT role_id FROM guild_support_roles WHERE guild_id = $1 ORDER BY role_id",
            as_i64(guild_id.get())
        ).fetch(pool).map_ok(|id| RoleId::new(as_u64(id))).try_collect()
        .await
    }

    pub async fn add(
        pool: &PgPool,
        guild_id: GuildId,
        role_id: RoleId,
    ) -> sqlx::Result<bool> {
        let guild_id = as_i64(guild_id.get());

        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        let inserted = sqlx::query!(
            "INSERT INTO guild_support_roles (guild_id, role_id) VALUES ($1, $2) \
             ON CONFLICT (guild_id, role_id) DO NOTHING",
            guild_id,
            as_i64(role_id.get())
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        tx.commit().await?;

        Ok(inserted == 1)
    }

    pub async fn remove(
        pool: &PgPool,
        guild_id: GuildId,
        role_id: RoleId,
    ) -> sqlx::Result<bool> {
        let deleted = sqlx::query!(
            "DELETE FROM guild_support_roles WHERE guild_id = $1 AND role_id = $2",
            as_i64(guild_id.get()),
            as_i64(role_id.get())
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(deleted == 1)
    }
}
