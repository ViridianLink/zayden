use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::dto::GreetingImageInfo,
    crate::dto::{CooldownView, Tier},
    crate::server::auth::{admin_guild_id, app_state, db_pool, server_err},
    crate::server::command_permissions::{
        MAX_ALLOWED_CHANNELS,
        channel_allowlist,
        command_id,
        fetch,
        guild_context,
        store,
        with_channel_allowlist,
    },
    crate::server::tier::guild_server_tier,
    greetings::{
        Cooldowns,
        GreetingImage,
        GreetingKind,
        GreetingsSettings,
        GreetingsSettingsRow,
        GuildId,
        parse_cooldown,
    },
    twilight_model::id::Id,
    twilight_model::id::marker::ChannelMarker,
};

#[cfg(feature = "ssr")]
const COMMAND: &str = "good";

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

#[cfg(feature = "ssr")]
fn parse_channel(raw: &str) -> Result<Id<ChannelMarker>, ServerFnError> {
    raw.trim()
        .parse::<u64>()
        .ok()
        .and_then(Id::new_checked)
        .ok_or_else(|| invalid("channel id"))
}

#[cfg(feature = "ssr")]
async fn edit_allowlist<F>(guild: &str, edit: F) -> Result<(), ServerFnError>
where
    F: FnOnce(&mut Vec<Id<ChannelMarker>>) -> Result<(), ServerFnError>,
{
    let ctx = guild_context(guild).await?;
    let cmd = command_id(&ctx, COMMAND).await?;

    let current = fetch(&ctx, cmd).await;
    let mut allowed = channel_allowlist(ctx.guild_id, &current);

    edit(&mut allowed)?;

    let updated = with_channel_allowlist(ctx.guild_id, &current, &allowed);

    store(&ctx, cmd, COMMAND, &updated).await
}

#[cfg(feature = "ssr")]
const fn floors_for(tier: Tier) -> Cooldowns {
    let tier = tier.as_entitlement();
    GreetingsSettingsRow::floors_for(tier)
}

#[server]
pub async fn get_greetings(guild: String) -> Result<GreetingsView, ServerFnError> {
    let guild_id = admin_guild_id(&guild).await?;
    let app = app_state()?;
    let pool = db_pool()?;

    let guild_id = GuildId::new(guild_id.cast_unsigned());

    let config = GreetingsSettings::get(&app.settings.greetings, guild_id)
        .await
        .map_err(server_err)?;

    let tier = guild_server_tier(guild_id.get()).await?;
    let floor = floors_for(tier);
    let next_tier = tier.next_paid();
    let next_floor = next_tier.map_or(floor, floors_for);

    let ctx = guild_context(&guild).await?;
    let allowed_channels = match command_id(&ctx, COMMAND).await {
        Ok(cmd) => channel_allowlist(ctx.guild_id, &fetch(&ctx, cmd).await)
            .into_iter()
            .map(|id| id.to_string())
            .collect(),
        // The command is not registered for this guild yet
        Err(_e) => Vec::new(),
    };

    Ok(GreetingsView {
        morning_message: config.morning_message.unwrap_or_default(),
        night_message: config.night_message.unwrap_or_default(),
        morning: images(&pool, guild_id, GreetingKind::Morning).await?,
        night: images(&pool, guild_id, GreetingKind::Night).await?,
        allowed_channels,
        channels_locked: !ctx.access.can_write_command_permissions(),
        cooldowns: CooldownView {
            user_secs: config.cooldowns.user_secs,
            guild_secs: config.cooldowns.guild_secs,
            floor_user_secs: floor.user_secs,
            floor_guild_secs: floor.guild_secs,
            tier,
            next_tier,
            next_floor_user_secs: next_floor.user_secs,
            next_floor_guild_secs: next_floor.guild_secs,
        },
    })
}

#[server]
pub async fn save_greeting_messages(
    guild: String,
    morning_message: String,
    night_message: String,
) -> Result<(), ServerFnError> {
    let guild_id = admin_guild_id(&guild).await?;
    let app = app_state()?;

    GreetingsSettings::save_messages(
        &app.settings.greetings,
        GuildId::new(guild_id.cast_unsigned()),
        &morning_message,
        &night_message,
    )
    .await
    .map_err(server_err)
}

#[cfg(feature = "ssr")]
fn check_floor(
    requested: i32,
    floor: i32,
    next_floor: i32,
    label: &str,
    tier: Tier,
) -> Result<(), ServerFnError> {
    if requested >= floor {
        return Ok(());
    }

    let upgrade = tier.next_paid().map_or_else(
        || "That is as low as this command goes.".to_string(),
        |next| format!("{} servers can go as low as {next_floor}s.", next.label()),
    );

    Err(ServerFnError::ServerError(format!(
        "On the {} plan the {label} cooldown can't go below {floor}s. {upgrade}",
        tier.label(),
    )))
}

#[server]
pub async fn save_greeting_cooldowns(
    guild: String,
    user_cooldown: String,
    guild_cooldown: String,
) -> Result<(), ServerFnError> {
    let guild_id = admin_guild_id(&guild).await?;
    let app = app_state()?;

    let guild_id = GuildId::new(guild_id.cast_unsigned());
    let tier = guild_server_tier(guild_id.get()).await?;
    let floor = floors_for(tier);
    let next_floor = tier.next_paid().map_or(floor, floors_for);

    let requested = Cooldowns {
        user_secs: parse_cooldown(&user_cooldown, floor.user_secs)
            .map_err(server_err)?,
        guild_secs: parse_cooldown(&guild_cooldown, floor.guild_secs)
            .map_err(server_err)?,
    };

    check_floor(
        requested.user_secs,
        floor.user_secs,
        next_floor.user_secs,
        "per-member",
        tier,
    )?;
    check_floor(
        requested.guild_secs,
        floor.guild_secs,
        next_floor.guild_secs,
        "server-wide",
        tier,
    )?;

    GreetingsSettings::save_cooldowns(
        &app.settings.greetings,
        guild_id,
        requested,
        floor,
    )
    .await
    .map(|_| ())
    .map_err(server_err)
}

#[server]
pub async fn add_greeting_channel(
    guild: String,
    channel_id: String,
) -> Result<(), ServerFnError> {
    let channel = parse_channel(&channel_id)?;

    edit_allowlist(&guild, |allowed| {
        if allowed.contains(&channel) {
            return Err(ServerFnError::ServerError(
                "that channel is already on the list".to_string(),
            ));
        }

        if allowed.len() >= MAX_ALLOWED_CHANNELS {
            return Err(ServerFnError::ServerError(format!(
                "Discord allows at most {MAX_ALLOWED_CHANNELS} channels per \
                 command. Remove one before adding another."
            )));
        }

        allowed.push(channel);

        Ok(())
    })
    .await
}

#[server]
pub async fn remove_greeting_channel(
    guild: String,
    channel_id: String,
) -> Result<(), ServerFnError> {
    let channel = parse_channel(&channel_id)?;

    edit_allowlist(&guild, |allowed| {
        let before = allowed.len();
        allowed.retain(|existing| *existing != channel);

        if allowed.len() == before {
            return Err(ServerFnError::ServerError(
                "that channel is not on this server's list".to_string(),
            ));
        }

        Ok(())
    })
    .await
}

#[server]
pub async fn add_greeting_image(
    guild: String,
    kind: String,
    url: String,
) -> Result<(), ServerFnError> {
    let guild_id = admin_guild_id(&guild).await?;
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
    let guild_id = admin_guild_id(&guild).await?;
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
