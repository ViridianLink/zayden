use jellyfin::index::LibraryItemRow;
use jiff::Timestamp;
use jiff_sqlx::Timestamp as SqlxTimestamp;
use serenity::all::{GenericChannelId, GuildId, MessageId, UserId};
use sqlx::PgPool;
use zayden_core::as_i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyState {
    Scheduled,
    Provisioned,
    Running,
    Cancelled,
    Cleaned,
}

impl PartyState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Provisioned => "provisioned",
            Self::Running => "running",
            Self::Cancelled => "cancelled",
            Self::Cleaned => "cleaned",
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PartyRow {
    pub id: i64,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: Option<i64>,
    pub event_id: Option<i64>,
    pub host_id: i64,
    pub item_id: String,
    pub item_name: String,
    pub item_type: String,
    pub item_parent_path: String,
    pub guests_enabled: bool,
    pub library_name: Option<String>,
    pub library_item_id: Option<String>,
    pub starts_at: SqlxTimestamp,
    pub ends_at: SqlxTimestamp,
    pub cleanup_after: SqlxTimestamp,
    pub state: String,
    pub cleaned_at: Option<SqlxTimestamp>,
}

impl PartyRow {
    #[must_use]
    pub fn starts_at(&self) -> Timestamp {
        self.starts_at.to_jiff()
    }

    #[must_use]
    pub fn cleanup_after(&self) -> Timestamp {
        self.cleanup_after.to_jiff()
    }

    #[must_use]
    pub const fn channel(&self) -> GenericChannelId {
        GenericChannelId::new(self.channel_id.cast_unsigned())
    }

    #[must_use]
    pub const fn host(&self) -> UserId {
        UserId::new(self.host_id.cast_unsigned())
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.state == PartyState::Cancelled.as_str()
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "an insert mirrors its table; grouping these into a struct would only move the argument list"
    )]
    #[expect(
        trivial_casts,
        reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
    )]
    pub async fn insert(
        pool: &PgPool,
        guild_id: GuildId,
        channel_id: GenericChannelId,
        host_id: UserId,
        item: &LibraryItemRow,
        item_parent_path: &str,
        guests_enabled: bool,
        starts_at: Timestamp,
        ends_at: Timestamp,
        cleanup_after: Timestamp,
    ) -> sqlx::Result<i64> {
        sqlx::query_scalar!(
            r#"
            INSERT INTO jellyfin_parties
                (guild_id, channel_id, host_id, item_id, item_name, item_type,
                 item_parent_path, guests_enabled, starts_at, ends_at,
                 cleanup_after)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id
            "#,
            as_i64(guild_id.get()),
            as_i64(channel_id.get()),
            as_i64(host_id.get()),
            item.item_id,
            item.name,
            item.item_type,
            item_parent_path,
            guests_enabled,
            SqlxTimestamp::from(starts_at) as SqlxTimestamp,
            SqlxTimestamp::from(ends_at) as SqlxTimestamp,
            SqlxTimestamp::from(cleanup_after) as SqlxTimestamp,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn get(pool: &PgPool, id: i64) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(Self, r#"SELECT id, guild_id, channel_id, message_id, event_id, host_id, item_id, item_name, item_type, item_parent_path, guests_enabled, library_name, library_item_id, starts_at AS "starts_at: SqlxTimestamp", ends_at AS "ends_at: SqlxTimestamp", cleanup_after AS "cleanup_after: SqlxTimestamp", state, cleaned_at AS "cleaned_at: SqlxTimestamp" FROM jellyfin_parties WHERE id = $1"#, id)
            .fetch_optional(pool)
            .await
    }

    pub async fn upcoming(
        pool: &PgPool,
        guild_id: GuildId,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(Self, r#"SELECT id, guild_id, channel_id, message_id, event_id, host_id, item_id, item_name, item_type, item_parent_path, guests_enabled, library_name, library_item_id, starts_at AS "starts_at: SqlxTimestamp", ends_at AS "ends_at: SqlxTimestamp", cleanup_after AS "cleanup_after: SqlxTimestamp", state, cleaned_at AS "cleaned_at: SqlxTimestamp" FROM jellyfin_parties WHERE guild_id = $1 AND state IN ('scheduled', 'provisioned', 'running') ORDER BY starts_at"#, as_i64(guild_id.get()))
            .fetch_all(pool)
            .await
    }

    pub async fn live(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(Self, r#"SELECT id, guild_id, channel_id, message_id, event_id, host_id, item_id, item_name, item_type, item_parent_path, guests_enabled, library_name, library_item_id, starts_at AS "starts_at: SqlxTimestamp", ends_at AS "ends_at: SqlxTimestamp", cleanup_after AS "cleanup_after: SqlxTimestamp", state, cleaned_at AS "cleaned_at: SqlxTimestamp" FROM jellyfin_parties WHERE cleaned_at IS NULL AND state <> 'cancelled' ORDER BY starts_at"#)
            .fetch_all(pool)
            .await
    }

    pub async fn due_for_cleanup(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(Self, r#"SELECT id, guild_id, channel_id, message_id, event_id, host_id, item_id, item_name, item_type, item_parent_path, guests_enabled, library_name, library_item_id, starts_at AS "starts_at: SqlxTimestamp", ends_at AS "ends_at: SqlxTimestamp", cleanup_after AS "cleanup_after: SqlxTimestamp", state, cleaned_at AS "cleaned_at: SqlxTimestamp" FROM jellyfin_parties WHERE cleanup_after < now() AND cleaned_at IS NULL AND state <> 'cancelled'"#)
            .fetch_all(pool)
            .await
    }

    pub async fn set_message(
        pool: &PgPool,
        id: i64,
        message_id: MessageId,
        event_id: Option<i64>,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_parties SET message_id = $2, event_id = $3 WHERE id = $1",
            id,
            as_i64(message_id.get()),
            event_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn claim_library_name(
        pool: &PgPool,
        id: i64,
        name: &str,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_parties SET library_name = $2 WHERE id = $1 AND library_name IS NULL",
            id,
            name
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn set_library_item(
        pool: &PgPool,
        id: i64,
        library_item_id: &str,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_parties SET library_item_id = $2, state = 'provisioned', provisioned_at = now() WHERE id = $1",
            id,
            library_item_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn set_state(
        pool: &PgPool,
        id: i64,
        state: PartyState,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_parties SET state = $2 WHERE id = $1",
            id,
            state.as_str()
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_cleaned(pool: &PgPool, id: i64) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_parties SET state = 'cleaned', cleaned_at = now() WHERE id = $1",
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PartyGuestRow {
    pub party_id: i64,
    pub user_id: i64,
    pub jellyfin_username: String,
    pub jellyfin_user_id: Option<String>,
    pub deleted_at: Option<SqlxTimestamp>,
}

impl PartyGuestRow {
    #[must_use]
    pub const fn user(&self) -> UserId {
        UserId::new(self.user_id.cast_unsigned())
    }

    pub async fn join(
        pool: &PgPool,
        party_id: i64,
        user_id: UserId,
        username: &str,
    ) -> sqlx::Result<()> {
        let discord_id = as_i64(user_id.get());

        sqlx::query!(
            "INSERT INTO users (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            discord_id
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "INSERT INTO jellyfin_party_guests (party_id, user_id, jellyfin_username) VALUES ($1, $2, $3) ON CONFLICT (party_id, user_id) DO NOTHING",
            party_id,
            discord_id,
            username
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn leave(
        pool: &PgPool,
        party_id: i64,
        user_id: UserId,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM jellyfin_party_guests WHERE party_id = $1 AND user_id = $2 AND jellyfin_user_id IS NULL",
            party_id,
            as_i64(user_id.get())
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn for_party(pool: &PgPool, party_id: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r#"SELECT party_id, user_id, jellyfin_username, jellyfin_user_id, deleted_at AS "deleted_at: SqlxTimestamp" FROM jellyfin_party_guests WHERE party_id = $1"#,
            party_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn live(pool: &PgPool, party_id: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r#"SELECT party_id, user_id, jellyfin_username, jellyfin_user_id, deleted_at AS "deleted_at: SqlxTimestamp" FROM jellyfin_party_guests WHERE party_id = $1 AND jellyfin_user_id IS NOT NULL AND deleted_at IS NULL"#,
            party_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn live_count(pool: &PgPool, guild_id: GuildId) -> sqlx::Result<i64> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM jellyfin_party_guests AS g
            JOIN jellyfin_parties AS p ON p.id = g.party_id
            WHERE p.guild_id = $1 AND g.jellyfin_user_id IS NOT NULL
              AND g.deleted_at IS NULL
            "#,
            as_i64(guild_id.get())
        )
        .fetch_one(pool)
        .await?;

        Ok(count.unwrap_or(0))
    }

    pub async fn set_provisioned(
        pool: &PgPool,
        party_id: i64,
        user_id: i64,
        jellyfin_user_id: &str,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_party_guests SET jellyfin_user_id = $3, provisioned_at = now() WHERE party_id = $1 AND user_id = $2",
            party_id,
            user_id,
            jellyfin_user_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_deleted(
        pool: &PgPool,
        party_id: i64,
        user_id: i64,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_party_guests SET deleted_at = now() WHERE party_id = $1 AND user_id = $2",
            party_id,
            user_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
