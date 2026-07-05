use serenity::all::{CreateComponent, EditInteractionResponse};

use super::MusicCtx;
use crate::components::ControlPanel;
use crate::embeds;
use crate::error::{MusicError, Result};

pub(super) async fn run(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;
    let guard = player.lock().await;
    let now = guard.current.as_ref().ok_or(MusicError::NothingPlaying)?;
    let embed = embeds::now_playing_embed(now, guard.loop_mode);
    drop(guard);

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().embed(embed).components(vec![
                CreateComponent::ActionRow(ControlPanel::buttons()),
            ]),
        )
        .await?;

    Ok(())
}
