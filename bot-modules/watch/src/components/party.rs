use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::runtime::JellyfinRuntime;
use jellyfin::settings::JellyfinSettings;
use serenity::all::EditInteractionResponse;
use zayden_core::ComponentCtx;

use crate::embeds;
use crate::error::{Result, WatchError};
use crate::party::row::{PartyGuestRow, PartyRow};
use crate::party::{Scheduled, lifecycle};

pub async fn join(
    cx: &ComponentCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    suffix: &str,
) -> Result<()> {
    let party_id = parse_id(suffix)?;
    let party = load(cx, party_id).await?;
    let guild_id = party.guild_id;

    let settings = JellyfinSettings::get(
        &cx.app.settings.jellyfin,
        serenity::all::GuildId::new(guild_id.cast_unsigned()),
    )
    .await
    .map_err(WatchError::Jellyfin)?;

    if !settings.guests_enabled {
        return Err(JellyfinError::GuestsDisabled.into());
    }

    let live = PartyGuestRow::live_count(
        &cx.app.db,
        serenity::all::GuildId::new(guild_id.cast_unsigned()),
    )
    .await?;

    if live >= i64::from(settings.max_concurrent_guests) {
        return Err(JellyfinError::GuestLimit {
            limit: settings.max_concurrent_guests,
        }
        .into());
    }

    let user = &cx.interaction.user;
    PartyGuestRow::join(&cx.app.db, party_id, user.id, user.name.as_str()).await?;

    cx.ephemeral(
        "You are on the list. If you have not linked a Jellyfin account I will \
         DM you temporary access 30 minutes before the party starts.",
    )
    .await?;

    refresh_embed(cx, runtime, party_id).await
}

pub async fn leave(
    cx: &ComponentCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    suffix: &str,
) -> Result<()> {
    let party_id = parse_id(suffix)?;
    load(cx, party_id).await?;

    let removed =
        PartyGuestRow::leave(&cx.app.db, party_id, cx.interaction.user.id).await?;

    let message = if removed {
        "Removed you from the party."
    } else {
        "You were not on the list, or your access has already been created — \
         ask the host if you need it revoked."
    };

    cx.ephemeral(message).await?;
    refresh_embed(cx, runtime, party_id).await
}

pub async fn cancel(
    cx: &ComponentCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    suffix: &str,
) -> Result<Scheduled> {
    let party_id = parse_id(suffix)?;
    let party = load(cx, party_id).await?;

    let is_host = party.host() == cx.interaction.user.id;
    let is_mod = cx
        .interaction
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .is_some_and(|p| p.manage_guild() || p.administrator());

    if !is_host && !is_mod {
        return Err(JellyfinError::NotPartyHost.into());
    }

    lifecycle::cancel(runtime, &cx.app.db, party_id).await?;

    cx.interaction.defer(&cx.ctx.http).await?;
    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new()
                .content(format!(
                    "Watch party for **{}** cancelled.",
                    party.item_name
                ))
                .embeds(vec![])
                .components(vec![]),
        )
        .await?;

    Ok(Scheduled::Clear(party_id))
}

async fn load(cx: &ComponentCtx<'_>, party_id: i64) -> Result<PartyRow> {
    PartyRow::get(&cx.app.db, party_id)
        .await?
        .ok_or_else(|| JellyfinError::NoSuchParty(party_id).into())
}

fn parse_id(suffix: &str) -> Result<i64> {
    suffix
        .parse()
        .map_err(|_e| WatchError::Internal(format!("bad party id `{suffix}`")))
}

async fn refresh_embed(
    cx: &ComponentCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    party_id: i64,
) -> Result<()> {
    let Some(party) = PartyRow::get(&cx.app.db, party_id).await? else {
        return Ok(());
    };
    let guests = PartyGuestRow::for_party(&cx.app.db, party_id).await?.len();

    let mut message = cx.interaction.message.clone();
    let edit = serenity::all::EditMessage::new().embed(embeds::party::embed(
        &runtime.jellyfin,
        &party,
        guests,
    ));

    message.edit(&cx.ctx.http, edit).await?;
    Ok(())
}
