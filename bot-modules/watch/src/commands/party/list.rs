use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{CreateEmbed, EditInteractionResponse};
use zayden_core::InvocationCtx;

use crate::embeds::COLOUR;
use crate::error::{Result, WatchError};
use crate::party::row::PartyRow;

pub async fn run(
    cx: &InvocationCtx<'_>,
    _runtime: &Arc<JellyfinRuntime>,
) -> Result<()> {
    let guild_id = cx.interaction.guild_id.ok_or(WatchError::MissingGuildId)?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let parties = PartyRow::upcoming(&cx.app.db, guild_id).await?;

    let body = if parties.is_empty() {
        "Nothing scheduled. Start one with `/watch party create`.".to_owned()
    } else {
        parties
            .iter()
            .map(|p| {
                format!(
                    "**#{}** {} — <t:{}:R> in <#{}>",
                    p.id,
                    p.item_name,
                    p.starts_at().as_second(),
                    p.channel_id.cast_unsigned()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().embed(
                CreateEmbed::new()
                    .title("Upcoming watch parties")
                    .colour(COLOUR)
                    .description(body),
            ),
        )
        .await?;

    Ok(())
}
