use greetings::{
    Cooldowns,
    GreetingImage,
    GreetingKind,
    GreetingsSettings,
    GreetingsSettingsRow,
    GuildId,
    parse_cooldown,
};
use sqlx::PgPool;
use topcoat::context::Cx;
use twilight_model::id::Id;
use twilight_model::id::marker::ChannelMarker;

use super::dto::{CooldownView, GreetingImageInfo, GreetingsView};
use super::error::EngagementError;
use super::form::form_args;
use super::parse::guild_ref;
use crate::auth::{
    ChannelInfo,
    GuildIds,
    admin_guild_id,
    app_state,
    db_pool,
    list_guild_channels,
};
use crate::guild::command_permissions::{
    GuildContext,
    MAX_ALLOWED_CHANNELS,
    channel_allowlist,
    command_id,
    fetch,
    guild_context,
    lookup_command_id,
    store,
    with_channel_allowlist,
};
use crate::guild::dto::Tier;
use crate::guild::tier::guild_server_tier;

const COMMAND: &str = "good";

form_args! {
    SaveGreetingMessagesForm { guild, morning_message, night_message }
}

form_args! {
    SaveGreetingCooldownsForm { guild, user_cooldown, guild_cooldown }
}

form_args! {
    GreetingChannelForm { guild, channel_id }
}

form_args! {
    AddGreetingImageForm { guild, kind, url }
}

form_args! {
    RemoveGreetingImageForm { guild, id }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreetingsPage {
    pub view: GreetingsView,
    pub channels: Vec<ChannelInfo>,
}

async fn images(
    pool: &PgPool,
    guild_id: GuildId,
    kind: GreetingKind,
) -> Result<Vec<GreetingImageInfo>, EngagementError> {
    let rows = GreetingImage::list(pool, guild_id, kind).await?;

    Ok(rows
        .into_iter()
        .map(|row| GreetingImageInfo { id: row.id.to_string(), url: row.url })
        .collect())
}

fn parse_channel(raw: &str) -> Result<Id<ChannelMarker>, EngagementError> {
    raw.trim()
        .parse::<u64>()
        .ok()
        .and_then(Id::new_checked)
        .ok_or(EngagementError::Invalid("channel id"))
}

async fn edit_allowlist<F>(
    ctx: &GuildContext,
    edit: F,
) -> Result<(), EngagementError>
where
    F: FnOnce(&mut Vec<Id<ChannelMarker>>) -> Result<(), EngagementError>,
{
    let cmd = command_id(ctx, COMMAND).await?;

    let current = fetch(ctx, cmd).await?;
    let mut allowed = channel_allowlist(ctx.guild_id, &current);

    edit(&mut allowed)?;

    let updated = with_channel_allowlist(ctx.guild_id, &current, &allowed);

    store(ctx, cmd, COMMAND, &updated).await?;

    Ok(())
}

async fn read_allowed_channels(
    ctx: &GuildContext,
) -> Result<Vec<String>, EngagementError> {
    let Some(cmd) = lookup_command_id(ctx, COMMAND).await? else {
        return Ok(Vec::new());
    };

    Ok(channel_allowlist(ctx.guild_id, &fetch(ctx, cmd).await?)
        .into_iter()
        .map(|id| id.to_string())
        .collect())
}

const fn floors_for(tier: Tier) -> Cooldowns {
    GreetingsSettingsRow::floors_for(tier.as_entitlement())
}

/// The guild's greeting messages, images, `/good` channel restrictions and
/// cooldowns. Channel restrictions that Discord would not report read as
/// `None`.
pub async fn get_greetings(
    cx: &Cx,
    guild: &str,
) -> Result<GreetingsView, EngagementError> {
    let guild_id = admin_guild_id(cx, guild).await?;
    let app = app_state(cx)?;
    let pool = db_pool(cx)?;

    let guild_id = guild_ref(guild_id)?;

    let config = GreetingsSettings::get(&app.settings.greetings, guild_id).await?;

    let tier = guild_server_tier(cx, guild_id.get()).await?;
    let floor = floors_for(tier);
    let next_tier = tier.next_paid();
    let next_floor = next_tier.map_or(floor, floors_for);

    let ctx = guild_context(cx, guild).await?;
    let allowed_channels = match read_allowed_channels(&ctx).await {
        Ok(channels) => Some(channels),
        Err(e) => {
            tracing::warn!(
                error = ?e,
                "failed to read /good channel restrictions from Discord; \
                 reporting them as unknown"
            );
            None
        },
    };

    Ok(GreetingsView {
        morning_message: config.morning_message.unwrap_or_default(),
        night_message: config.night_message.unwrap_or_default(),
        morning: images(pool, guild_id, GreetingKind::Morning).await?,
        night: images(pool, guild_id, GreetingKind::Night).await?,
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

/// The greetings view plus the guild's channels for the pickers and chips. A
/// failed channel listing reads as an empty one.
pub async fn load_greetings_page(
    cx: &Cx,
    guild: &str,
) -> Result<GreetingsPage, EngagementError> {
    let view = get_greetings(cx, guild).await?;
    let channels = list_guild_channels(cx, guild).await.unwrap_or_default();

    Ok(GreetingsPage { view, channels })
}

/// Saves both greeting messages; a blank one clears it.
pub async fn save_greeting_messages(
    cx: &Cx,
    form: &SaveGreetingMessagesForm,
) -> Result<(), EngagementError> {
    let guild_id = admin_guild_id(cx, &form.guild).await?;
    let app = app_state(cx)?;

    GreetingsSettings::save_messages(
        &app.settings.greetings,
        guild_ref(guild_id)?,
        &form.morning_message,
        &form.night_message,
    )
    .await?;

    Ok(())
}

fn check_floor(
    requested: i32,
    floor: i32,
    next_floor: i32,
    label: &'static str,
    tier: Tier,
) -> Result<(), EngagementError> {
    if requested >= floor {
        return Ok(());
    }

    let upgrade = tier.next_paid().map_or_else(
        || "That is as low as this command goes.".to_string(),
        |next| format!("{} servers can go as low as {next_floor}s.", next.label()),
    );

    Err(EngagementError::CooldownBelowFloor {
        tier: tier.label(),
        label,
        floor,
        upgrade,
    })
}

/// Saves the per-member and server-wide cooldowns; a blank one resets to the
/// plan's floor, and neither may go below it.
pub async fn save_greeting_cooldowns(
    cx: &Cx,
    form: &SaveGreetingCooldownsForm,
) -> Result<(), EngagementError> {
    let guild_id = admin_guild_id(cx, &form.guild).await?;
    let app = app_state(cx)?;

    let guild_id = guild_ref(guild_id)?;
    let tier = guild_server_tier(cx, guild_id.get()).await?;
    let floor = floors_for(tier);
    let next_floor = tier.next_paid().map_or(floor, floors_for);

    let requested = Cooldowns {
        user_secs: parse_cooldown(&form.user_cooldown, floor.user_secs)?,
        guild_secs: parse_cooldown(&form.guild_cooldown, floor.guild_secs)?,
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
    .await?;

    Ok(())
}

/// Allows `/good` in one more channel, writing the guild's command
/// permissions with the signed-in member's own token.
pub async fn add_greeting_channel(
    cx: &Cx,
    form: &GreetingChannelForm,
) -> Result<(), EngagementError> {
    let channel = parse_channel(&form.channel_id)?;
    let ctx = guild_context(cx, &form.guild).await?;

    GuildIds::default()
        .channel(Some(channel.get().cast_signed()))
        .ensure_in(cx, ctx.guild_id.get().cast_signed())
        .await?;

    edit_allowlist(&ctx, |allowed| {
        if allowed.contains(&channel) {
            return Err(EngagementError::ChannelAlreadyListed);
        }

        if allowed.len() >= MAX_ALLOWED_CHANNELS {
            return Err(EngagementError::ChannelListFull(MAX_ALLOWED_CHANNELS));
        }

        allowed.push(channel);

        Ok(())
    })
    .await
}

/// Stops allowing `/good` in a channel. Removing the last one lifts the
/// restriction.
pub async fn remove_greeting_channel(
    cx: &Cx,
    form: &GreetingChannelForm,
) -> Result<(), EngagementError> {
    let channel = parse_channel(&form.channel_id)?;
    let ctx = guild_context(cx, &form.guild).await?;

    edit_allowlist(&ctx, |allowed| {
        let before = allowed.len();
        allowed.retain(|existing| *existing != channel);

        if allowed.len() == before {
            return Err(EngagementError::ChannelNotListed);
        }

        Ok(())
    })
    .await
}

pub async fn add_greeting_image(
    cx: &Cx,
    form: &AddGreetingImageForm,
) -> Result<(), EngagementError> {
    let guild_id = admin_guild_id(cx, &form.guild).await?;
    let pool = db_pool(cx)?;

    let kind = GreetingKind::parse(&form.kind)?;

    GreetingImage::add(pool, guild_ref(guild_id)?, kind, &form.url).await?;

    Ok(())
}

pub async fn remove_greeting_image(
    cx: &Cx,
    form: &RemoveGreetingImageForm,
) -> Result<(), EngagementError> {
    let guild_id = admin_guild_id(cx, &form.guild).await?;
    let pool = db_pool(cx)?;

    let id = form
        .id
        .trim()
        .parse::<i32>()
        .map_err(|_e| EngagementError::Invalid("image id"))?;

    let removed = GreetingImage::remove(pool, guild_ref(guild_id)?, id).await?;

    if removed { Ok(()) } else { Err(EngagementError::ImageNotFound) }
}
