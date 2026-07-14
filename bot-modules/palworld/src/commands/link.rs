use std::collections::HashMap;

use serenity::all::ResolvedValue;
use sqlx::PgPool;
use zayden_core::{InvocationCtx, as_i64, required_option};

use super::respond;
use crate::client::PalworldClient;
use crate::error::{PalworldError, Result};
use crate::link::PlayerLink;
use crate::{embeds, save};

pub(super) async fn link(
    cx: &InvocationCtx<'_>,
    client: &PalworldClient,
    pool: &PgPool,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let name: &str =
        required_option(&mut options, "name").map_err(PalworldError::from)?;

    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    let roster = client.roster().await?;
    let Some(player) = roster.by_name(name) else {
        let mut names: Vec<&str> =
            roster.players.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        return respond(cx, embeds::link_error_component(name, &names)).await;
    };

    let discord_id = as_i64(cx.interaction.user.id.get());
    let stored =
        PlayerLink::upsert(pool, discord_id, &player.uid, &player.name).await?;

    let pals = client.pals().await?;
    let owned = player
        .pals
        .iter()
        .filter(|p| save::palmap::resolve_species(&p.species, &pals).is_some())
        .count();

    respond(cx, embeds::link_component(&stored.in_game_name, owned)).await
}

pub(super) async fn unlink(cx: &InvocationCtx<'_>, pool: &PgPool) -> Result<()> {
    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    let discord_id = as_i64(cx.interaction.user.id.get());
    PlayerLink::delete(pool, discord_id).await?;

    respond(cx, embeds::unlink_component()).await
}
