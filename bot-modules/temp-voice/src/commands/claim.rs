use serenity::all::{
    ChannelId,
    CommandInteraction,
    Context,
    EditInteractionResponse,
};
use sqlx::PgPool;

use crate::{TempVoiceError, VoiceChannelRow, VoiceStateCache, actions};

pub(super) async fn claim(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    voice_states: &VoiceStateCache,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(&ctx.http).await?;

    let msg = actions::claim(
        &ctx.http,
        pool,
        voice_states,
        channel_id,
        row,
        interaction.user.id,
    )
    .await?;

    interaction
        .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
