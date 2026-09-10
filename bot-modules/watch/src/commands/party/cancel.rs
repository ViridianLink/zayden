use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, required_option};

use crate::error::Result;
use crate::party::row::PartyRow;
use crate::party::{Scheduled, lifecycle};

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<Scheduled> {
    let id: i64 = required_option(&mut options, "id")?;

    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    let party = PartyRow::get(&cx.app.db, id)
        .await?
        .ok_or(JellyfinError::NoSuchParty(id))?;

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

    lifecycle::cancel(runtime, &cx.app.db, id).await?;

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().content(format!(
                "Party #{id} cancelled, and any temporary access has been \
                 removed."
            )),
        )
        .await?;

    Ok(Scheduled::Clear(id))
}
