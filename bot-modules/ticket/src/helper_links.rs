use std::collections::HashMap;

use serenity::all::{GuildId, UserId};
use sqlx::PgPool;
use zayden_core::{as_i64, as_u64};

pub struct HelperLink {
    pub user_id: UserId,
    pub link: String,
}

pub struct HelperLinks;

impl HelperLinks {
    pub async fn list(
        pool: &PgPool,
        guild_id: GuildId,
    ) -> sqlx::Result<Vec<HelperLink>> {
        let rows = sqlx::query!(
            "SELECT user_id, link FROM guild_helper_links WHERE guild_id = $1 \
             ORDER BY user_id",
            as_i64(guild_id.get())
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| HelperLink {
                user_id: UserId::new(as_u64(r.user_id)),
                link: r.link,
            })
            .collect())
    }

    pub async fn map(
        pool: &PgPool,
        guild_id: GuildId,
    ) -> sqlx::Result<HashMap<UserId, String>> {
        Ok(Self::list(pool, guild_id)
            .await?
            .into_iter()
            .map(|l| (l.user_id, l.link))
            .collect())
    }

    pub async fn set(
        pool: &PgPool,
        guild_id: GuildId,
        user_id: UserId,
        link: &str,
    ) -> sqlx::Result<()> {
        let guild_id = as_i64(guild_id.get());

        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO guild_helper_links (guild_id, user_id, link) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (guild_id, user_id) DO UPDATE SET link = EXCLUDED.link",
            guild_id,
            as_i64(user_id.get()),
            link
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn remove(
        pool: &PgPool,
        guild_id: GuildId,
        user_id: UserId,
    ) -> sqlx::Result<bool> {
        let deleted = sqlx::query!(
            "DELETE FROM guild_helper_links WHERE guild_id = $1 AND user_id = $2",
            as_i64(guild_id.get()),
            as_i64(user_id.get())
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(deleted == 1)
    }
}
