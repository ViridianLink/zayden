mod slash_command;

use async_trait::async_trait;
use chrono_tz::Tz;
use lfg::commands::{JoinedManager, SetupManager};
use lfg::components::{EditManager, EditRow};
use lfg::modals::create::{GuildManager, GuildRow};
use lfg::models::timezone_manager::LOCALE_TO_TIMEZONE;
use lfg::{JoinedRow, PostManager, PostRow, Savable, TimezoneManager}; // PostRow
use serenity::all::{ChannelId, GenericChannelId, GuildId, MessageId, RoleId, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};

pub use slash_command::Lfg;

pub struct PostTable;

#[async_trait]
impl PostManager<Postgres> for PostTable {
    async fn exists(pool: &PgPool, id: impl Into<GenericChannelId> + Send) -> sqlx::Result<bool> {
        let id = id.into();

        sqlx::query_scalar!(
            "SELECT EXISTS (SELECT 1 FROM lfg_posts WHERE id = $1)",
            id.get() as i64
        )
        .fetch_one(pool)
        .await
        .map(|exists| exists.unwrap_or(false))
    }

    async fn owner(pool: &PgPool, id: impl Into<GenericChannelId> + Send) -> sqlx::Result<UserId> {
        let id = id.into();

        sqlx::query_scalar!("SELECT owner from lfg_posts WHERE id = $1", id.get() as i64)
            .fetch_one(pool)
            .await
            .map(|id| UserId::new(id as u64))
    }

    async fn row(pool: &PgPool, id: impl Into<GenericChannelId> + Send) -> sqlx::Result<PostRow> {
        let id = id.into();

        sqlx::query_as!(
            PostRow,
            r#"
            SELECT
                p.id,
                p.owner,
                p.activity,
                p.start_time,
                p.description,
                p.fireteam_size,

                COALESCE(
                    (SELECT array_agg(f.user_id) FROM lfg_fireteam f WHERE f.post = p.id),
                    '{}'
                ) AS "fireteam!",

                COALESCE(
                    (SELECT array_agg(a.user_id) FROM lfg_alternatives a WHERE a.post = p.id),
                    '{}'
                ) AS "alternatives!",

                m.message AS "alt_message?",
                m.channel AS "alt_channel?"

            FROM
                lfg_posts p
            LEFT JOIN
                lfg_messages m on p.id = m.id
            WHERE
                p.id = $1
            "#,
            id.get() as i64
        )
        .fetch_one(pool)
        .await
    }

    async fn delete(
        pool: &PgPool,
        id: impl Into<GenericChannelId> + Send,
    ) -> sqlx::Result<PgQueryResult> {
        let id = id.into();

        sqlx::query!("DELETE FROM lfg_posts WHERE id = $1", id.get() as i64)
            .execute(pool)
            .await
    }
}

async fn save_post(pool: &PgPool, row: PostRow) -> sqlx::Result<PgQueryResult> {
    let mut tx = pool.begin().await?;

    let mut result = sqlx::query!(
        r#"
        INSERT INTO lfg_posts (id, owner, activity, start_time, description, fireteam_size)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (id) DO UPDATE
        SET
            owner = EXCLUDED.owner,
            activity = EXCLUDED.activity,
            start_time = EXCLUDED.start_time,
            description = EXCLUDED.description,
            fireteam_size = EXCLUDED.fireteam_size;
        "#,
        row.id,
        row.owner,
        row.activity,
        row.start_time,
        row.description,
        row.fireteam_size
    )
    .execute(&mut *tx)
    .await?;

    let temp_result1 = sqlx::query!("DELETE FROM lfg_fireteam WHERE post = $1", row.id)
        .execute(&mut *tx)
        .await?;

    let temp_result2 = sqlx::query!("DELETE FROM lfg_alternatives WHERE post = $1", row.id)
        .execute(&mut *tx)
        .await?;

    result.extend([temp_result1, temp_result2]);

    if !row.fireteam.is_empty() {
        let temp_result = sqlx::query!("INSERT INTO lfg_fireteam (post, user_id) SELECT $1, user_id FROM UNNEST($2::bigint[]) AS t(user_id)", row.id, &row.fireteam)
                .execute(&mut *tx)
                .await?;

        result.extend([temp_result]);
    }

    if !row.alternatives.is_empty() {
        let temp_result = sqlx::query!("INSERT INTO lfg_alternatives (post, user_id) SELECT $1, user_id FROM UNNEST($2::bigint[]) AS t(user_id)", row.id, &row.alternatives)
                .execute(&mut *tx)
                .await?;

        result.extend([temp_result]);
    }

    if let (Some(channel), Some(message)) = (row.alt_channel, row.alt_message) {
        let temp_result = sqlx::query!(
            "INSERT INTO lfg_messages (id, message, channel) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
            row.id,
            message,
            channel,
        )
        .execute(&mut *tx)
        .await?;

        result.extend([temp_result]);
    }

    tx.commit().await.unwrap();

    Ok(result)
}

#[async_trait]
impl Savable<Postgres, PostRow> for PostTable {
    async fn save(pool: &PgPool, row: PostRow) -> sqlx::Result<PgQueryResult> {
        save_post(pool, row).await
    }
}

#[async_trait]
impl SetupManager<Postgres> for PostTable {
    async fn insert(
        pool: &PgPool,
        id: impl Into<GuildId> + Send,
        channel: impl Into<ChannelId> + Send,
        role: Option<impl Into<RoleId> + Send>,
    ) -> sqlx::Result<PgQueryResult> {
        let id = id.into();
        let channel = channel.into();
        let role = role.map(|role| role.into());

        sqlx::query!(
            r#"
            INSERT INTO lfg_guilds (id, channel_id, role_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE
            SET
                channel_id = EXCLUDED.channel_id,
                role_id = EXCLUDED.role_id;
            "#,
            id.get() as i64,
            channel.get() as i64,
            role.map(|role| role.get() as i64),
        )
        .execute(pool)
        .await
    }
}

#[async_trait]
impl JoinedManager<Postgres> for PostTable {
    async fn upcoming(
        pool: &PgPool,
        user: impl Into<UserId> + Send,
    ) -> sqlx::Result<Vec<JoinedRow>> {
        let user = user.into();

        sqlx::query_as!(
            JoinedRow,
            r#"
            SELECT
                p.id,
                p.activity,
                p.start_time,
            
                COALESCE(
                    (SELECT array_agg(f.user_id) FROM lfg_fireteam f WHERE f.post = p.id),
                    '{}'
                ) AS "fireteam!"
            
            FROM
                lfg_posts p
            JOIN lfg_fireteam f ON p.id = f.post
            WHERE
                f.user_id = $1
            "#,
            user.get() as i64
        )
        .fetch_all(pool)
        .await
    }
}

#[async_trait]
impl EditManager<Postgres> for PostTable {
    async fn edit_row(pool: &PgPool, id: impl Into<MessageId> + Send) -> sqlx::Result<EditRow> {
        let id = id.into();

        sqlx::query_as!(
            EditRow,
            r#"
            SELECT
                p.owner,
                p.activity,
                p.start_time,
                p.description,
                p.fireteam_size,
                u.timezone AS "timezone?"
            FROM
                lfg_posts AS p
            LEFT JOIN
                lfg_users AS u ON p.owner = u.id
            WHERE
                p.id = $1
            "#,
            id.get() as i64
        )
        .fetch_one(pool)
        .await
    }
}

pub struct UsersTable;

#[async_trait]
impl TimezoneManager<Postgres> for UsersTable {
    async fn get(pool: &PgPool, id: impl Into<UserId> + Send, local: &str) -> sqlx::Result<Tz> {
        let id = id.into();

        let tz = sqlx::query!(
            "SELECT timezone FROM lfg_users WHERE id = $1",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await?;

        match tz {
            Some(tz) => Ok(tz.timezone.parse().unwrap_or(chrono_tz::UTC)),
            None => Ok(LOCALE_TO_TIMEZONE
                .get(local)
                .copied()
                .unwrap_or(chrono_tz::UTC)),
        }
    }

    async fn save(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
        tz: Tz,
    ) -> sqlx::Result<PgQueryResult> {
        let id = id.into();

        sqlx::query!(
            "INSERT INTO lfg_users (id, timezone) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET timezone = $2",
            id.get() as i64,
            tz.name()
        )
        .execute(pool)
        .await
    }
}

pub struct GuildTable;

#[async_trait]
impl GuildManager<Postgres> for GuildTable {
    async fn row(pool: &PgPool, id: impl Into<GuildId> + Send) -> sqlx::Result<Option<GuildRow>> {
        let id = id.into();

        sqlx::query_as!(
            GuildRow,
            "SELECT channel_id, scheduled_thread_id FROM lfg_guilds WHERE id = $1",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }
}
