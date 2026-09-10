use jiff_sqlx::Timestamp;
use serenity::all::UserId;
use sqlx::PgPool;
use zayden_core::as_i64;

use crate::error::{JellyfinError, Result};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JellyfinLinkRow {
    pub user_id: i64,
    pub jellyfin_user_id: String,
    pub jellyfin_username: String,
    pub jellyseerr_user_id: Option<i32>,
    pub jellyseerr_checked_at: Option<Timestamp>,
    pub streak_public: bool,
    pub letterboxd_username: Option<String>,
}

impl JellyfinLinkRow {
    pub async fn get(pool: &PgPool, user_id: UserId) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT user_id, jellyfin_user_id, jellyfin_username,
                   jellyseerr_user_id,
                   jellyseerr_checked_at AS "jellyseerr_checked_at: Timestamp",
                   streak_public, letterboxd_username
            FROM jellyfin_users
            WHERE user_id = $1
            "#,
            as_i64(user_id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn require(pool: &PgPool, user_id: UserId) -> Result<Self> {
        Self::get(pool, user_id).await?.ok_or(JellyfinError::NotLinked)
    }

    pub async fn insert(
        pool: &PgPool,
        user_id: UserId,
        jellyfin_user_id: &str,
        jellyfin_username: &str,
    ) -> Result<()> {
        let discord_id = as_i64(user_id.get());

        sqlx::query!(
            "INSERT INTO users (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            discord_id
        )
        .execute(pool)
        .await?;

        let result = sqlx::query!(
            r#"
            INSERT INTO jellyfin_users
                (user_id, jellyfin_user_id, jellyfin_username)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE SET
                jellyfin_user_id = EXCLUDED.jellyfin_user_id,
                jellyfin_username = EXCLUDED.jellyfin_username,
                updated_at = now()
            "#,
            discord_id,
            jellyfin_user_id,
            jellyfin_username
        )
        .execute(pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(JellyfinError::AccountClaimed)
            },
            Err(e) => Err(e.into()),
        }
    }

    pub async fn delete(pool: &PgPool, user_id: UserId) -> sqlx::Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM jellyfin_users WHERE user_id = $1",
            as_i64(user_id.get())
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn set_streak_public(
        pool: &PgPool,
        user_id: UserId,
        visible: bool,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_users SET streak_public = $2, updated_at = now() \
             WHERE user_id = $1",
            as_i64(user_id.get()),
            visible
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_letterboxd(
        pool: &PgPool,
        user_id: UserId,
        username: Option<&str>,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_users SET letterboxd_username = $2, \
             updated_at = now() WHERE user_id = $1",
            as_i64(user_id.get()),
            username
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_jellyseerr_user(
        pool: &PgPool,
        user_id: i64,
        jellyseerr_user_id: Option<i32>,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE jellyfin_users SET jellyseerr_user_id = $2, \
             jellyseerr_checked_at = now(), updated_at = now() WHERE user_id = $1",
            user_id,
            jellyseerr_user_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn by_jellyfin_id(
        pool: &PgPool,
        jellyfin_user_id: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT user_id, jellyfin_user_id, jellyfin_username,
                   jellyseerr_user_id,
                   jellyseerr_checked_at AS "jellyseerr_checked_at: Timestamp",
                   streak_public, letterboxd_username
            FROM jellyfin_users
            WHERE jellyfin_user_id = $1
            "#,
            jellyfin_user_id
        )
        .fetch_optional(pool)
        .await
    }
}
