use jiff_sqlx::{Timestamp, ToSqlx};
use serenity::all::ChannelId;
use sqlx::PgPool;
use zayden_core::as_u64;

use crate::error::Result;
use crate::model::PatreonPost;
use crate::oauth::TokenPair;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PatreonCampaignRow {
    pub campaign_id: String,
    pub next_cursor: Option<String>,
    pub seeded_at: Option<Timestamp>,
    pub consecutive_failures: i32,
}

impl PatreonCampaignRow {
    #[must_use]
    pub const fn is_seeded(&self) -> bool {
        self.seeded_at.is_some()
    }

    pub async fn select(pool: &PgPool, campaign_id: &str) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"
            SELECT campaign_id,
                   next_cursor,
                   seeded_at AS "seeded_at: Timestamp",
                   consecutive_failures
            FROM patreon_campaigns
            WHERE campaign_id = $1
            "#,
            campaign_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn ensure(pool: &PgPool, campaign_id: &str) -> Result<()> {
        sqlx::query!(
            "INSERT INTO patreon_campaigns (campaign_id) VALUES ($1)
             ON CONFLICT (campaign_id) DO NOTHING",
            campaign_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn record_success(
        pool: &PgPool,
        campaign_id: &str,
        next_cursor: Option<&str>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE patreon_campaigns
            SET next_cursor = $2,
                seeded_at = COALESCE(seeded_at, now()),
                last_polled_at = now(),
                consecutive_failures = 0
            WHERE campaign_id = $1
            "#,
            campaign_id,
            next_cursor
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn record_failure(pool: &PgPool, campaign_id: &str) -> Result<i32> {
        let failures = sqlx::query_scalar!(
            r#"
            UPDATE patreon_campaigns
            SET last_polled_at = now(), consecutive_failures = consecutive_failures + 1
            WHERE campaign_id = $1
            RETURNING consecutive_failures
            "#,
            campaign_id
        )
        .fetch_one(pool)
        .await?;

        Ok(failures)
    }
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn insert_post(
    pool: &PgPool,
    post: &PatreonPost,
    announced: bool,
) -> Result<bool> {
    let inserted = sqlx::query_scalar!(
        r#"
        INSERT INTO patreon_posts (
            post_id, campaign_id, title, url, content_html, is_public,
            published_at, announced_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, CASE WHEN $8 THEN now() END)
        ON CONFLICT (post_id) DO NOTHING
        RETURNING TRUE AS "inserted!"
        "#,
        post.id,
        post.campaign_id,
        post.title,
        post.url,
        post.content_html,
        post.is_public,
        post.published_at.to_sqlx() as Timestamp,
        announced
    )
    .fetch_optional(pool)
    .await?;

    Ok(inserted.is_some())
}

#[derive(Debug, Clone)]
pub struct PatreonConnection {
    pub guild_id: i64,
    pub campaign_id: String,
    pub creator_name: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Timestamp,
    pub webhook_id: Option<String>,
    pub disabled_at: Option<Timestamp>,
}

impl PatreonConnection {
    pub async fn select(pool: &PgPool, guild_id: i64) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id,
                   campaign_id,
                   creator_name,
                   access_token,
                   refresh_token,
                   expires_at AS "expires_at: Timestamp",
                   webhook_id,
                   disabled_at AS "disabled_at: Timestamp"
            FROM patreon_oauth
            WHERE guild_id = $1
            "#,
            guild_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn pollable(pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"
            SELECT DISTINCT ON (o.campaign_id)
                   o.guild_id,
                   o.campaign_id,
                   o.creator_name,
                   o.access_token,
                   o.refresh_token,
                   o.expires_at AS "expires_at: Timestamp",
                   o.webhook_id,
                   o.disabled_at AS "disabled_at: Timestamp"
            FROM patreon_oauth o
            JOIN patreon_announce a ON a.guild_id = o.guild_id
            WHERE o.disabled_at IS NULL
            ORDER BY o.campaign_id, o.guild_id
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn connect(
        pool: &PgPool,
        guild_id: i64,
        campaign_id: &str,
        creator_name: Option<&str>,
        connected_by: i64,
        tokens: &TokenPair,
        webhook: Option<(&str, &str)>,
    ) -> Result<()> {
        let (webhook_id, webhook_secret) = match webhook {
            Some((id, secret)) => (Some(id), Some(secret)),
            None => (None, None),
        };

        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO patreon_campaigns (campaign_id) VALUES ($1)
             ON CONFLICT (campaign_id) DO NOTHING",
            campaign_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO patreon_oauth (
                guild_id, campaign_id, creator_name, access_token, refresh_token,
                expires_at, webhook_id, webhook_secret, connected_by
            )
            VALUES ($1, $2, $3, $4, $5, now() + make_interval(secs => $6), $7, $8, $9)
            ON CONFLICT (guild_id) DO UPDATE
            SET campaign_id = EXCLUDED.campaign_id,
                creator_name = EXCLUDED.creator_name,
                access_token = EXCLUDED.access_token,
                refresh_token = EXCLUDED.refresh_token,
                expires_at = EXCLUDED.expires_at,
                webhook_id = EXCLUDED.webhook_id,
                webhook_secret = EXCLUDED.webhook_secret,
                connected_by = EXCLUDED.connected_by,
                disabled_at = NULL,
                updated_at = now()
            "#,
            guild_id,
            campaign_id,
            creator_name,
            tokens.access_token,
            tokens.refresh_token,
            tokens.expires_in,
            webhook_id,
            webhook_secret,
            connected_by
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn store_tokens(
        pool: &PgPool,
        guild_id: i64,
        tokens: &TokenPair,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE patreon_oauth
            SET access_token = $2,
                refresh_token = $3,
                expires_at = now() + make_interval(secs => $4),
                disabled_at = NULL,
                updated_at = now()
            WHERE guild_id = $1
            "#,
            guild_id,
            tokens.access_token,
            tokens.refresh_token,
            tokens.expires_in
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn disable(pool: &PgPool, guild_id: i64) -> Result<()> {
        sqlx::query!(
            "UPDATE patreon_oauth SET disabled_at = now(), updated_at = now()
             WHERE guild_id = $1",
            guild_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &PgPool, guild_id: i64) -> Result<bool> {
        let deleted =
            sqlx::query!("DELETE FROM patreon_oauth WHERE guild_id = $1", guild_id)
                .execute(pool)
                .await?
                .rows_affected();

        Ok(deleted > 0)
    }
}

pub async fn webhook_secrets(
    pool: &PgPool,
    campaign_id: &str,
) -> Result<Vec<String>> {
    let secrets = sqlx::query_scalar!(
        r#"
        SELECT webhook_secret AS "webhook_secret!"
        FROM patreon_oauth
        WHERE campaign_id = $1 AND webhook_secret IS NOT NULL AND disabled_at IS NULL
        "#,
        campaign_id
    )
    .fetch_all(pool)
    .await?;

    Ok(secrets)
}

pub async fn is_subscribed(pool: &PgPool, campaign_id: &str) -> Result<bool> {
    let subscribed = sqlx::query_scalar!(
        r#"SELECT EXISTS (
            SELECT 1
            FROM patreon_oauth o
            JOIN patreon_announce a ON a.guild_id = o.guild_id
            WHERE o.campaign_id = $1 AND o.disabled_at IS NULL
        ) AS "exists!""#,
        campaign_id
    )
    .fetch_one(pool)
    .await?;

    Ok(subscribed)
}

#[derive(Debug, Clone)]
pub struct PendingPost {
    pub post_id: String,
    pub campaign_id: String,
    pub title: Option<String>,
    pub url: String,
    pub content_html: Option<String>,
    pub thumbnail_url: Option<String>,
    pub is_public: bool,
    pub published_at: Timestamp,
}

pub async fn claim_pending(pool: &PgPool, limit: i64) -> Result<Vec<PendingPost>> {
    let rows = sqlx::query_as!(
        PendingPost,
        r#"
        WITH due AS (
            SELECT p.post_id
            FROM patreon_posts p
            WHERE p.announced_at IS NULL
            ORDER BY p.campaign_id, p.published_at
            LIMIT $1
            FOR UPDATE OF p SKIP LOCKED
        )
        UPDATE patreon_posts p
        SET announced_at = now()
        FROM due
        WHERE p.post_id = due.post_id
        RETURNING p.post_id,
                  p.campaign_id,
                  p.title,
                  p.url,
                  p.content_html,
                  p.thumbnail_url,
                  p.is_public,
                  p.published_at AS "published_at: Timestamp"
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn set_thumbnail(pool: &PgPool, post_id: &str, url: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE patreon_posts SET thumbnail_url = $2 WHERE post_id = $1",
        post_id,
        url
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PatreonAnnounceRow {
    pub guild_id: i64,
    pub channel_id: i64,
    pub public_only: bool,
}

impl PatreonAnnounceRow {
    #[must_use]
    pub const fn channel(&self) -> ChannelId {
        ChannelId::new(as_u64(self.channel_id))
    }

    pub async fn for_post(
        pool: &PgPool,
        campaign_id: &str,
        is_public: bool,
    ) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"
            SELECT a.guild_id, a.channel_id, a.public_only
            FROM patreon_announce a
            JOIN patreon_oauth o ON o.guild_id = a.guild_id
            WHERE o.campaign_id = $1
              AND o.disabled_at IS NULL
              AND (NOT a.public_only OR $2)
            "#,
            campaign_id,
            is_public
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn select(pool: &PgPool, guild_id: i64) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            "SELECT guild_id, channel_id, public_only
             FROM patreon_announce WHERE guild_id = $1",
            guild_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn upsert(
        pool: &PgPool,
        guild_id: i64,
        channel_id: i64,
        public_only: bool,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO patreon_announce (guild_id, channel_id, public_only)
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id) DO UPDATE
            SET channel_id = EXCLUDED.channel_id,
                public_only = EXCLUDED.public_only,
                updated_at = now()
            "#,
            guild_id,
            channel_id,
            public_only
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &PgPool, guild_id: i64) -> Result<bool> {
        let deleted = sqlx::query!(
            "DELETE FROM patreon_announce WHERE guild_id = $1",
            guild_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(deleted > 0)
    }
}
