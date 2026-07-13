use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};

use crate::{TempVoiceError, VoiceChannelRow, actions};

pub(super) async fn bitrate(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    let Some(ResolvedValue::Integer(kbps)) = options.remove("kbps") else {
        return Err(TempVoiceError::IneligibleChannel);
    };

    let msg =
        actions::bitrate(http, channel_id, row, interaction.user.id, kbps).await?;

    interaction
        .edit_response(http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
