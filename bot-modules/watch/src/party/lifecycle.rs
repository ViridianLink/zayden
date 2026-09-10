use std::sync::Arc;

use jellyfin::guest::{self, naming};
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{CreateMessage, Http, UserId};
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::error::Result;
use crate::party::deeplink;
use crate::party::row::{PartyGuestRow, PartyRow, PartyState};

pub async fn provision(
    http: &Http,
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    party_id: i64,
) -> Result<()> {
    let Some(party) = PartyRow::get(pool, party_id).await? else {
        return Ok(());
    };

    if party.is_cancelled() || party.cleaned_at.is_some() {
        return Ok(());
    }

    if !party.guests_enabled {
        PartyRow::set_state(pool, party_id, PartyState::Provisioned).await?;
        return Ok(());
    }

    let pending: Vec<PartyGuestRow> = PartyGuestRow::for_party(pool, party_id)
        .await?
        .into_iter()
        .filter(|g| g.jellyfin_user_id.is_none() && g.deleted_at.is_none())
        .collect();

    if pending.is_empty() {
        PartyRow::set_state(pool, party_id, PartyState::Provisioned).await?;
        return Ok(());
    }

    // Name first, create second: the unique index then makes a retry adopt the
    // existing library instead of creating a duplicate.
    let library_name = naming::library_name(party_id);
    PartyRow::claim_library_name(pool, party_id, &library_name).await?;

    let library_item_id = guest::ensure_library(
        runtime,
        party_id,
        collection_type(&party.item_type),
        &party.item_parent_path,
    )
    .await?;

    PartyRow::set_library_item(pool, party_id, &library_item_id).await?;

    for row in pending {
        let user_id = row.user();

        let provisioned =
            guest::create_guest(runtime, party_id, user_id.get(), &library_item_id)
                .await;

        let provisioned = match provisioned {
            Ok(p) => p,
            Err(e) => {
                error!(error = ?e, party_id, %user_id, "could not provision guest");
                continue;
            },
        };

        PartyGuestRow::set_provisioned(
            pool,
            party_id,
            row.user_id,
            &provisioned.jellyfin_user_id,
        )
        .await?;

        dm_credentials(http, runtime, &party, user_id, &provisioned).await;
    }

    info!(party_id, "jellyfin party provisioned");
    Ok(())
}

fn collection_type(item_type: &str) -> &'static str {
    if item_type == "Movie" { "movies" } else { "tvshows" }
}

async fn dm_credentials(
    http: &Http,
    runtime: &Arc<JellyfinRuntime>,
    party: &PartyRow,
    user_id: UserId,
    guest: &guest::ProvisionedGuest,
) {
    let content = format!(
        "Your temporary access for **{}**\n\
         Username: `{}`\nPassword: `{}`\n\n{}",
        party.item_name,
        guest.username,
        guest.password,
        deeplink::join_guide(&runtime.jellyfin, party),
    );

    if let Err(e) = user_id.dm(http, CreateMessage::new().content(content)).await {
        warn!(error = ?e, %user_id, "could not DM party credentials");
    }
}

pub async fn cleanup(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    party_id: i64,
) -> Result<()> {
    let Some(party) = PartyRow::get(pool, party_id).await? else {
        return Ok(());
    };

    if party.cleaned_at.is_some() {
        return Ok(());
    }

    for row in PartyGuestRow::live(pool, party_id).await? {
        let Some(jellyfin_user_id) = row.jellyfin_user_id.as_deref() else {
            continue;
        };

        match guest::delete_guest(runtime, jellyfin_user_id).await {
            Ok(()) => {
                PartyGuestRow::mark_deleted(pool, party_id, row.user_id).await?;
            },
            Err(e) => {
                error!(error = ?e, party_id, "could not delete guest");
            },
        }
    }

    if party.library_name.is_some()
        && let Err(e) = guest::delete_library(runtime, party_id).await
    {
        error!(error = ?e, party_id, "could not delete party library");
    }

    PartyRow::mark_cleaned(pool, party_id).await?;
    info!(party_id, "jellyfin party cleaned up");

    Ok(())
}

pub async fn cancel(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    party_id: i64,
) -> Result<()> {
    cleanup(runtime, pool, party_id).await?;
    PartyRow::set_state(pool, party_id, PartyState::Cancelled).await?;
    Ok(())
}

pub async fn announce(
    http: &Http,
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    party_id: i64,
    content: String,
) {
    let Ok(Some(party)) = PartyRow::get(pool, party_id).await else {
        return;
    };

    if party.is_cancelled() {
        return;
    }

    let link = deeplink::watch_link(&runtime.jellyfin, &party);
    let message = CreateMessage::new().content(format!("{content}\n{link}"));

    if let Err(e) = party.channel().send_message(http, message).await {
        warn!(error = ?e, party_id, "could not post party reminder");
    }
}
