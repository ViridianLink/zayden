use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    Context,
    EditInteractionResponse,
    GuildId,
    ResolvedValue,
};

use crate::{TempVoiceError, VoiceChannelRow, VoiceStateCache, actions};

pub(super) async fn privacy(
    ctx: &Context,
    interaction: &CommandInteraction,
    voice_states: &VoiceStateCache,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    guild_id: GuildId,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(&ctx.http).await?;

    let privacy = match options.remove("privacy") {
        Some(ResolvedValue::String(privacy)) => privacy,
        _ => "visible",
    };

    let msg = actions::privacy(
        &ctx.http,
        guild_id,
        voice_states,
        channel_id,
        &row,
        interaction.user.id,
        privacy,
    )
    .await?;

    interaction
        .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
