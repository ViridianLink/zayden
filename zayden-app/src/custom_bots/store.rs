use sqlx::PgPool;

use super::{CustomBotError, SealedToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomBotStatus {
    Active,
    Revoked,
}

impl CustomBotStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NewCustomBot<'a> {
    pub application_id: i64,
    pub bot_user_id: i64,
    pub name: &'a str,
    pub avatar: Option<&'a str>,
    pub public_key: &'a str,
    pub registered_by: i64,
}

pub struct CustomBotStore;

impl CustomBotStore {
    pub async fn upsert(
        pool: &PgPool,
        bot: &NewCustomBot<'_>,
        sealed: &SealedToken,
    ) -> Result<(), CustomBotError> {
        sqlx::query!(
            "INSERT INTO custom_bots
                 (application_id, bot_user_id, name, avatar, public_key,
                  token_ct, token_nonce, key_id, registered_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (application_id) DO UPDATE SET
                 bot_user_id = EXCLUDED.bot_user_id,
                 name = EXCLUDED.name,
                 avatar = EXCLUDED.avatar,
                 public_key = EXCLUDED.public_key,
                 token_ct = EXCLUDED.token_ct,
                 token_nonce = EXCLUDED.token_nonce,
                 key_id = EXCLUDED.key_id,
                 status = 'active',
                 last_error = NULL,
                 token_rotated_at = now()",
            bot.application_id,
            bot.bot_user_id,
            bot.name,
            bot.avatar,
            bot.public_key,
            sealed.ciphertext,
            sealed.nonce,
            sealed.key_id,
            bot.registered_by,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn attach(
        pool: &PgPool,
        guild_id: i64,
        application_id: i64,
        attached_by: i64,
    ) -> Result<(), CustomBotError> {
        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO custom_bot_guilds (guild_id, application_id, attached_by)
             VALUES ($1, $2, $3)
             ON CONFLICT (guild_id) DO NOTHING",
            guild_id,
            application_id,
            attached_by,
        )
        .execute(&mut *tx)
        .await?;

        let attached = sqlx::query_scalar!(
            "SELECT application_id FROM custom_bot_guilds WHERE guild_id = $1",
            guild_id
        )
        .fetch_one(&mut *tx)
        .await?;

        if attached != application_id {
            return Err(CustomBotError::GuildTaken {
                guild_id,
                application_id: attached,
            });
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn detach(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<bool, CustomBotError> {
        let detached = sqlx::query!(
            "DELETE FROM custom_bot_guilds WHERE guild_id = $1",
            guild_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(detached > 0)
    }

    pub async fn guilds(
        pool: &PgPool,
        application_id: i64,
    ) -> Result<Vec<i64>, CustomBotError> {
        let guilds = sqlx::query_scalar!(
            "SELECT guild_id FROM custom_bot_guilds
             WHERE application_id = $1 ORDER BY guild_id",
            application_id
        )
        .fetch_all(pool)
        .await?;

        Ok(guilds)
    }

    pub async fn sealed_token(
        pool: &PgPool,
        application_id: i64,
    ) -> Result<Option<SealedToken>, CustomBotError> {
        let sealed = sqlx::query!(
            "SELECT key_id, token_nonce, token_ct FROM custom_bots
             WHERE application_id = $1",
            application_id
        )
        .fetch_optional(pool)
        .await?
        .map(|row| SealedToken {
            key_id: row.key_id,
            nonce: row.token_nonce,
            ciphertext: row.token_ct,
        });

        Ok(sealed)
    }

    pub async fn set_status(
        pool: &PgPool,
        application_id: i64,
        status: CustomBotStatus,
    ) -> Result<(), CustomBotError> {
        sqlx::query!(
            "UPDATE custom_bots SET status = $2 WHERE application_id = $1",
            application_id,
            status.as_str(),
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
