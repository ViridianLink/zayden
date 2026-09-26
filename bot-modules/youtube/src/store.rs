use jiff_sqlx::{Timestamp, ToSqlx};
use serenity::all::ChannelId;
use sqlx::PgPool;
use zayden_core::as_u64;

use crate::error::Result;
use crate::model::{OwnChannel, YoutubeVideo};

#[derive(Debug, Clone)]
pub struct YoutubeChannelRow {
    pub channel_id: String,
    pub title: String,
    pub uploads_playlist_id: String,
    pub websub_secret: String,
    pub seeded_at: Option<Timestamp>,
}

impl YoutubeChannelRow {
    #[must_use]
    pub const fn is_seeded(&self) -> bool {
        self.seeded_at.is_some()
    }

    pub async fn select(pool: &PgPool, channel_id: &str) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"
            SELECT channel_id,
                   title,
                   uploads_playlist_id,
                   websub_secret,
                   seeded_at AS "seeded_at: Timestamp"
            FROM youtube_channels
            WHERE channel_id = $1
            "#,
            channel_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn pollable(pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"
            SELECT c.channel_id,
                   c.title,
                   c.uploads_playlist_id,
                   c.websub_secret,
                   c.seeded_at AS "seeded_at: Timestamp"
            FROM youtube_channels c
            WHERE EXISTS (
                SELECT 1
                FROM youtube_connections o
                JOIN youtube_announce a ON a.guild_id = o.guild_id
                WHERE o.channel_id = c.channel_id
            )
            ORDER BY c.channel_id
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn lease_due(pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"
            SELECT c.channel_id,
                   c.title,
                   c.uploads_playlist_id,
                   c.websub_secret,
                   c.seeded_at AS "seeded_at: Timestamp"
            FROM youtube_channels c
            WHERE (c.websub_expires_at IS NULL
                   OR c.websub_expires_at < now() + interval '1 day')
              AND EXISTS (
                  SELECT 1 FROM youtube_connections o
                  WHERE o.channel_id = c.channel_id
              )
            ORDER BY c.channel_id
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn record_success(pool: &PgPool, channel_id: &str) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE youtube_channels
            SET seeded_at = COALESCE(seeded_at, now()),
                last_polled_at = now(),
                consecutive_failures = 0
            WHERE channel_id = $1
            "#,
            channel_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn record_failure(pool: &PgPool, channel_id: &str) -> Result<i32> {
        let failures = sqlx::query_scalar!(
            r#"
            UPDATE youtube_channels
            SET last_polled_at = now(), consecutive_failures = consecutive_failures + 1
            WHERE channel_id = $1
            RETURNING consecutive_failures
            "#,
            channel_id
        )
        .fetch_one(pool)
        .await?;

        Ok(failures)
    }

    pub async fn set_lease(
        pool: &PgPool,
        channel_id: &str,
        lease_seconds: i64,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE youtube_channels
            SET websub_expires_at = now() + make_interval(secs => $2::bigint::double precision)
            WHERE channel_id = $1
            "#,
            channel_id,
            lease_seconds
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn clear_lease(pool: &PgPool, channel_id: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE youtube_channels SET websub_expires_at = NULL
             WHERE channel_id = $1",
            channel_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn forget_videos(pool: &PgPool, channel_id: &str) -> Result<u64> {
        let mut tx = pool.begin().await?;

        let unseeded = sqlx::query!(
            r#"
            UPDATE youtube_channels c
            SET seeded_at = NULL
            WHERE c.channel_id = $1
              AND NOT EXISTS (
                  SELECT 1 FROM youtube_connections o WHERE o.channel_id = c.channel_id
              )
            "#,
            channel_id
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if unseeded == 0 {
            return Ok(0);
        }

        let deleted = sqlx::query!(
            "DELETE FROM youtube_videos WHERE channel_id = $1",
            channel_id
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        tx.commit().await?;

        Ok(deleted)
    }
}

#[derive(Debug, Clone)]
pub struct YoutubeConnection {
    pub guild_id: i64,
    pub channel_id: String,
    pub channel_title: String,
    pub lease_active: bool,
}

impl YoutubeConnection {
    pub async fn select(pool: &PgPool, guild_id: i64) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"
            SELECT o.guild_id,
                   o.channel_id,
                   c.title AS channel_title,
                   COALESCE(c.websub_expires_at > now(), FALSE) AS "lease_active!"
            FROM youtube_connections o
            JOIN youtube_channels c ON c.channel_id = o.channel_id
            WHERE o.guild_id = $1
            "#,
            guild_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn connect(
        pool: &PgPool,
        guild_id: i64,
        channel: &OwnChannel,
        connected_by: i64,
        new_secret: &str,
    ) -> Result<String> {
        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        let secret = sqlx::query_scalar!(
            r#"
            INSERT INTO youtube_channels (channel_id, title, uploads_playlist_id, websub_secret)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (channel_id) DO UPDATE
            SET title = EXCLUDED.title,
                uploads_playlist_id = EXCLUDED.uploads_playlist_id
            RETURNING websub_secret
            "#,
            channel.id,
            channel.title,
            channel.uploads_playlist_id,
            new_secret
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO youtube_connections (guild_id, channel_id, connected_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id) DO UPDATE
            SET channel_id = EXCLUDED.channel_id,
                connected_by = EXCLUDED.connected_by,
                updated_at = now()
            "#,
            guild_id,
            channel.id,
            connected_by
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(secret)
    }

    pub async fn delete(pool: &PgPool, guild_id: i64) -> Result<Option<String>> {
        let channel_id = sqlx::query_scalar!(
            "DELETE FROM youtube_connections WHERE guild_id = $1 RETURNING channel_id",
            guild_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(channel_id)
    }

    pub async fn channel_has_connections(
        pool: &PgPool,
        channel_id: &str,
    ) -> Result<bool> {
        let exists = sqlx::query_scalar!(
            r#"SELECT EXISTS (
                SELECT 1 FROM youtube_connections WHERE channel_id = $1
            ) AS "exists!""#,
            channel_id
        )
        .fetch_one(pool)
        .await?;

        Ok(exists)
    }
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn insert_video(
    pool: &PgPool,
    video: &YoutubeVideo,
    announced: bool,
) -> Result<bool> {
    let inserted = sqlx::query_scalar!(
        r#"
        INSERT INTO youtube_videos (video_id, channel_id, title, published_at, announced_at)
        VALUES ($1, $2, $3, $4, CASE WHEN $5 THEN now() END)
        ON CONFLICT (video_id) DO NOTHING
        RETURNING TRUE AS "inserted!"
        "#,
        video.id,
        video.channel_id,
        video.title,
        video.published_at.to_sqlx() as Timestamp,
        announced
    )
    .fetch_optional(pool)
    .await?;

    Ok(inserted.is_some())
}

pub async fn is_subscribed(pool: &PgPool, channel_id: &str) -> Result<bool> {
    let subscribed = sqlx::query_scalar!(
        r#"SELECT EXISTS (
            SELECT 1
            FROM youtube_connections o
            JOIN youtube_announce a ON a.guild_id = o.guild_id
            WHERE o.channel_id = $1
        ) AS "exists!""#,
        channel_id
    )
    .fetch_one(pool)
    .await?;

    Ok(subscribed)
}

#[derive(Debug, Clone)]
pub struct PendingVideo {
    pub video_id: String,
    pub channel_id: String,
    pub channel_title: String,
    pub title: String,
    pub published_at: Timestamp,
}

pub async fn claim_pending(pool: &PgPool, limit: i64) -> Result<Vec<PendingVideo>> {
    let rows = sqlx::query_as!(
        PendingVideo,
        r#"
        WITH due AS (
            SELECT v.video_id
            FROM youtube_videos v
            WHERE v.announced_at IS NULL
            ORDER BY v.channel_id, v.published_at
            LIMIT $1
            FOR UPDATE OF v SKIP LOCKED
        )
        UPDATE youtube_videos v
        SET announced_at = now()
        FROM due, youtube_channels c
        WHERE v.video_id = due.video_id AND c.channel_id = v.channel_id
        RETURNING v.video_id,
                  v.channel_id,
                  c.title AS channel_title,
                  v.title,
                  v.published_at AS "published_at: Timestamp"
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

#[derive(Debug, Clone)]
pub struct YoutubeAnnounceRow {
    pub guild_id: i64,
    pub channel_id: i64,
}

impl YoutubeAnnounceRow {
    #[must_use]
    pub const fn channel(&self) -> ChannelId {
        ChannelId::new(as_u64(self.channel_id))
    }

    pub async fn for_channel(
        pool: &PgPool,
        youtube_channel_id: &str,
    ) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"
            SELECT a.guild_id, a.channel_id
            FROM youtube_announce a
            JOIN youtube_connections o ON o.guild_id = a.guild_id
            WHERE o.channel_id = $1
            "#,
            youtube_channel_id
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn select(pool: &PgPool, guild_id: i64) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            "SELECT guild_id, channel_id FROM youtube_announce WHERE guild_id = $1",
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
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO youtube_announce (guild_id, channel_id)
            VALUES ($1, $2)
            ON CONFLICT (guild_id) DO UPDATE
            SET channel_id = EXCLUDED.channel_id,
                updated_at = now()
            "#,
            guild_id,
            channel_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &PgPool, guild_id: i64) -> Result<bool> {
        let deleted = sqlx::query!(
            "DELETE FROM youtube_announce WHERE guild_id = $1",
            guild_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(deleted > 0)
    }
}
