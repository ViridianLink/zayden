use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{CreateComponent, EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, required_option};

use crate::embeds;
use crate::error::Result;
use crate::party::row::{PartyGuestRow, PartyRow};

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let id: i64 = required_option(&mut options, "id")?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let party = PartyRow::get(&cx.app.db, id)
        .await?
        .ok_or(JellyfinError::NoSuchParty(id))?;
    let guests = PartyGuestRow::for_party(&cx.app.db, id).await?.len();

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new()
                .embed(embeds::party::embed(&runtime.jellyfin, &party, guests))
                .components(vec![CreateComponent::ActionRow(
                    embeds::party::buttons(id),
                )]),
        )
        .await?;

    Ok(())
}
