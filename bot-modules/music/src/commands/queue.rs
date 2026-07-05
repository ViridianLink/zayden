use std::collections::HashMap;

use serenity::all::{CreateComponent, EditInteractionResponse, ResolvedValue};

use super::MusicCtx;
use crate::components::QueuePager;
use crate::embeds;
use crate::error::{MusicError, Result};

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let page = match options.remove("page") {
        Some(ResolvedValue::Integer(n)) => usize::try_from(n.max(1) - 1).unwrap_or(0),
        _ => 0,
    };

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::QueueEmpty)?;
    let guard = player.lock().await;
    let current = guard.current.as_ref().map(|now| &now.track);
    let embed = embeds::queue_embed(&guard.queue, current, page);
    let total_pages = embeds::queue_page_count(guard.queue.len());
    drop(guard);

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new()
                .embed(embed)
                .components(vec![CreateComponent::ActionRow(QueuePager::buttons(page, total_pages))]),
        )
        .await?;

    Ok(())
}
