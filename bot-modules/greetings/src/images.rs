use serenity::all::GuildId;
use sqlx::PgPool;
use zayden_core::as_i64;

use crate::error::{GreetingsError, Result};
use crate::kind::GreetingKind;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GreetingImage {
    pub id: i32,
    pub guild_id: i64,
    pub kind: String,
    pub url: String,
}

impl GreetingImage {
    pub const MAX_PER_KIND: i64 = 50;

    pub async fn list(
        pool: &PgPool,
        guild_id: GuildId,
        kind: GreetingKind,
    ) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            "SELECT id, guild_id, kind, url
             FROM greeting_images
             WHERE guild_id = $1 AND kind = $2
             ORDER BY id",
            as_i64(guild_id.get()),
            kind.as_str()
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn add(
        pool: &PgPool,
        guild_id: GuildId,
        kind: GreetingKind,
        url: &str,
    ) -> Result<Self> {
        let url = validate_url(url)?;
        let guild_id = as_i64(guild_id.get());

        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!("SELECT id FROM guilds WHERE id = $1 FOR UPDATE", guild_id)
            .fetch_one(&mut *tx)
            .await?;

        let existing = sqlx::query!(
            r#"SELECT count(*) AS "count!"
               FROM greeting_images
               WHERE guild_id = $1 AND kind = $2"#,
            guild_id,
            kind.as_str()
        )
        .fetch_one(&mut *tx)
        .await?;

        if existing.count >= Self::MAX_PER_KIND {
            return Err(GreetingsError::TooManyImages(Self::MAX_PER_KIND));
        }

        let row = sqlx::query_as!(
            Self,
            "INSERT INTO greeting_images (guild_id, kind, url)
             VALUES ($1, $2, $3)
             RETURNING id, guild_id, kind, url",
            guild_id,
            kind.as_str(),
            &url
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            if e.as_database_error()
                .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
            {
                GreetingsError::DuplicateImage
            } else {
                GreetingsError::Sqlx(e)
            }
        })?;

        tx.commit().await?;

        Ok(row)
    }

    pub async fn remove(pool: &PgPool, guild_id: GuildId, id: i32) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM greeting_images WHERE id = $1 AND guild_id = $2",
            id,
            as_i64(guild_id.get())
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }
}

pub const MAX_URL_LEN: usize = 2048;

pub fn validate_url(raw: &str) -> Result<String> {
    let url = raw.trim();

    let invalid = || GreetingsError::InvalidUrl(url.to_string());

    if url.len() > MAX_URL_LEN {
        return Err(invalid());
    }

    let Some(rest) = url.strip_prefix("https://") else {
        return Err(invalid());
    };

    if rest.is_empty() {
        return Err(invalid());
    }

    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid());
    }

    Ok(url.to_string())
}
