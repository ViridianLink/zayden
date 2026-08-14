use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::dto::GreetingImageInfo,
    crate::server::auth::{app_state, db_pool, guild_admin_context, server_err},
    greetings::{
        GreetingImage,
        GreetingKind,
        GreetingsConfig,
        GreetingsSettings,
        GuildId,
    },
};

use crate::dto::GreetingsView;

#[cfg(feature = "ssr")]
fn invalid(what: &str) -> ServerFnError {
    ServerFnError::ServerError(format!("invalid {what}"))
}

#[cfg(feature = "ssr")]
async fn images(
    pool: &sqlx::PgPool,
    guild_id: GuildId,
    kind: GreetingKind,
) -> Result<Vec<GreetingImageInfo>, ServerFnError> {
    let rows =
        GreetingImage::list(pool, guild_id, kind).await.map_err(server_err)?;

    Ok(rows
        .into_iter()
        .map(|row| GreetingImageInfo { id: row.id.to_string(), url: row.url })
        .collect())
}

#[server]
pub async fn get_greetings(guild: String) -> Result<GreetingsView, ServerFnError> {
    let (guild_id, _user, _token) = guild_admin_context(&guild).await?;
    let app = app_state()?;
    let pool = db_pool()?;

    let guild_id = GuildId::new(guild_id.cast_unsigned());

    let config = GreetingsSettings::get(&app.settings.greetings, guild_id)
        .await
        .map_err(server_err)?;

    Ok(GreetingsView {
        morning_message: config.morning_message.unwrap_or_default(),
        night_message: config.night_message.unwrap_or_default(),
        morning: images(&pool, guild_id, GreetingKind::Morning).await?,
        night: images(&pool, guild_id, GreetingKind::Night).await?,
    })
}

#[server]
pub async fn save_greeting_messages(
    guild: String,
    morning_message: String,
    night_message: String,
) -> Result<(), ServerFnError> {
    let (guild_id, _user, _token) = guild_admin_context(&guild).await?;
    let app = app_state()?;

    let config = GreetingsConfig::from_form(&morning_message, &night_message)
        .map_err(server_err)?;

    GreetingsSettings::save(
        &app.settings.greetings,
        GuildId::new(guild_id.cast_unsigned()),
        config,
    )
    .await
    .map(|_| ())
    .map_err(server_err)
}

#[server]
pub async fn add_greeting_image(
    guild: String,
    kind: String,
    url: String,
) -> Result<(), ServerFnError> {
    let (guild_id, _user, _token) = guild_admin_context(&guild).await?;
    let pool = db_pool()?;

    let kind = GreetingKind::parse(&kind).map_err(server_err)?;

    GreetingImage::add(&pool, GuildId::new(guild_id.cast_unsigned()), kind, &url)
        .await
        .map(|_| ())
        .map_err(server_err)
}

#[server]
pub async fn remove_greeting_image(
    guild: String,
    id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, _user, _token) = guild_admin_context(&guild).await?;
    let pool = db_pool()?;

    let id = id.trim().parse::<i32>().map_err(|_e| invalid("image id"))?;

    let removed =
        GreetingImage::remove(&pool, GuildId::new(guild_id.cast_unsigned()), id)
            .await
            .map_err(server_err)?;

    if removed {
        Ok(())
    } else {
        Err(ServerFnError::ServerError(
            "that image is not in this server's list".to_string(),
        ))
    }
}
