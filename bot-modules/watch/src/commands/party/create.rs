use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::runtime::JellyfinRuntime;
use jellyfin::settings::JellyfinSettings;
use jellyfin::transport::jellyfin::model::ticks_to_seconds;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};
use serenity::all::{
    CreateComponent,
    CreateMessage,
    EditInteractionResponse,
    GenericChannelId,
    GenericInteractionChannel,
    ResolvedValue,
};
use sqlx::PgPool;
use zayden_core::{InvocationCtx, UserTimezone, optional_option, required_option};

use crate::discovery::resolve_local;
use crate::embeds;
use crate::error::{Result, WatchError};
use crate::party::Scheduled;
use crate::party::row::{PartyGuestRow, PartyRow};

const CLEANUP_GRACE_HOURS: i64 = 2;
const FALLBACK_RUNTIME_MINUTES: i64 = 150;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<Scheduled> {
    let title: &str = required_option(&mut options, "title")?;
    let when: &str = required_option(&mut options, "when")?;
    let guests = optional_option::<bool, _>(&mut options, "guests").unwrap_or(true);
    let channel: Option<&GenericInteractionChannel> =
        optional_option(&mut options, "channel");

    let guild_id = cx.interaction.guild_id.ok_or(WatchError::MissingGuildId)?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let settings =
        JellyfinSettings::get(&cx.app.settings.jellyfin, guild_id).await?;

    let item = resolve_local(&cx.app.db, title, None).await?;
    let root = item
        .library_root()
        .ok_or_else(|| JellyfinError::NoItemPath(item.name.clone()))?
        .to_owned();

    let starts_at = parse_when(&cx.app.db, cx, when).await?;
    if starts_at <= Timestamp::now() {
        return Err(JellyfinError::PartyInThePast.into());
    }

    let runtime_seconds =
        item.runtime_ticks.map_or(FALLBACK_RUNTIME_MINUTES * 60, ticks_to_seconds);
    let ends_at = starts_at + Span::new().seconds(runtime_seconds);
    let cleanup_after = ends_at + Span::new().hours(CLEANUP_GRACE_HOURS);

    let target = channel
        .map_or(cx.interaction.channel_id, |c| c.id().expect_channel().widen());
    let target = settings.party_channel_id.unwrap_or(target);

    let party_id = PartyRow::insert(
        &cx.app.db,
        guild_id,
        target,
        cx.interaction.user.id,
        &item,
        &root,
        guests && settings.guests_enabled,
        starts_at,
        ends_at,
        cleanup_after,
    )
    .await?;

    let Some(party) = PartyRow::get(&cx.app.db, party_id).await? else {
        return Err(WatchError::Internal("party vanished after insert".to_owned()));
    };

    let message = post_announcement(cx, runtime, &party, target).await?;
    PartyRow::set_message(&cx.app.db, party_id, message, None).await?;

    let guest_note = if party.guests_enabled {
        "Anyone without a Jellyfin account can press Join and I will DM them \
         temporary access 30 minutes beforehand."
    } else {
        "Guest accounts are off in this server, so everyone needs their own \
         Jellyfin login."
    };

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().content(format!(
                "Watch party #{party_id} for **{}** is up in <#{target}>.\n\
                 {guest_note}\n\n\
                 I cannot drive SyncPlay for you — when it starts, whoever hosts \
                 should open a SyncPlay group in their own client and everyone \
                 else joins it from the cast menu.",
                party.item_name
            )),
        )
        .await?;

    Ok(Scheduled::Party(Box::new(party)))
}

async fn post_announcement(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    party: &PartyRow,
    channel: GenericChannelId,
) -> Result<serenity::all::MessageId> {
    let guests = PartyGuestRow::for_party(&cx.app.db, party.id).await?.len();

    let message = channel
        .send_message(
            &cx.ctx.http,
            CreateMessage::new()
                .embed(embeds::party::embed(&runtime.jellyfin, party, guests))
                .components(vec![CreateComponent::ActionRow(
                    embeds::party::buttons(party.id),
                )]),
        )
        .await?;

    Ok(message.id)
}

async fn parse_when(
    pool: &PgPool,
    cx: &InvocationCtx<'_>,
    raw: &str,
) -> Result<Timestamp> {
    let tz = UserTimezone::get(pool, cx.interaction.user.id, &cx.interaction.locale)
        .await
        .unwrap_or(TimeZone::UTC);

    if let Ok(timestamp) = raw.parse::<Timestamp>() {
        return Ok(timestamp);
    }

    let civil: DateTime = raw
        .parse()
        .or_else(|_e| format!("{raw}:00").parse())
        .or_else(|_e| format!("{raw} 00:00:00").parse())
        .map_err(|_e: jiff::Error| JellyfinError::BadTime(raw.to_owned()))?;

    civil
        .to_zoned(tz)
        .map(|zoned| zoned.timestamp())
        .map_err(|_e| JellyfinError::BadTime(raw.to_owned()).into())
}
