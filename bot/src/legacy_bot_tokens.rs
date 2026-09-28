//! Temporary: adopts the plaintext custom bot tokens in `bot_tokens` into
//! encrypted `custom_bots`. Delete this module together with the `bot_tokens`
//! table once every legacy row has been adopted.

use secrecy::{ExposeSecret, SecretString};
use serenity::all::{Http, Token};
use sqlx::PgPool;
use tracing::{info, warn};
use zayden_app::custom_bots::{CustomBotStore, Keyring, NewCustomBot};
use zayden_core::as_i64;

use crate::Result;

pub(crate) fn spawn(pool: PgPool, registered_by: u64) {
    tokio::spawn(async move {
        if let Err(e) = adopt_all(&pool, as_i64(registered_by)).await {
            warn!(error = ?e, "failed to adopt legacy bot tokens");
        }
    });
}

async fn adopt_all(pool: &PgPool, registered_by: i64) -> Result<()> {
    let rows =
        sqlx::query!("SELECT name, token FROM bot_tokens WHERE name <> 'zayden'")
            .fetch_all(pool)
            .await?;

    if rows.is_empty() {
        return Ok(());
    }

    let keyring = match Keyring::from_env() {
        Ok(keyring) => keyring,
        Err(e) => {
            warn!(error = %e, "legacy bot tokens are waiting to be adopted");
            return Ok(());
        },
    };

    for row in rows {
        let token = SecretString::from(row.token);
        if let Err(e) = adopt(pool, &keyring, &row.name, &token, registered_by).await
        {
            warn!(name = row.name, error = ?e, "failed to adopt a legacy bot token");
        }
    }

    Ok(())
}

async fn adopt(
    pool: &PgPool,
    keyring: &Keyring,
    name: &str,
    token: &SecretString,
    registered_by: i64,
) -> Result<()> {
    let http = Http::new(
        token.expose_secret().parse::<Token>().map_err(serenity::Error::Token)?,
    );
    let (user, application) = tokio::try_join!(
        http.get_current_user(),
        http.get_current_application_info(),
    )?;

    let application_id = as_i64(application.id.get());
    let avatar = user.avatar.map(|hash| hash.to_string());
    let bot = NewCustomBot {
        application_id,
        bot_user_id: as_i64(user.id.get()),
        name: &user.name,
        avatar: avatar.as_deref(),
        public_key: &application.verify_key,
        registered_by,
    };
    CustomBotStore::upsert(pool, &bot, &keyring.seal(application_id, token)?)
        .await?;

    let guilds = sqlx::query_scalar!(
        "SELECT guild_id FROM guild_presence
         WHERE application_id = $1 AND left_at IS NULL",
        application_id
    )
    .fetch_all(pool)
    .await?;

    for &guild_id in &guilds {
        CustomBotStore::attach(pool, guild_id, application_id, registered_by)
            .await?;
    }

    sqlx::query!("DELETE FROM bot_tokens WHERE name = $1", name)
        .execute(pool)
        .await?;

    info!(name, application_id, ?guilds, "adopted legacy bot token");
    Ok(())
}
