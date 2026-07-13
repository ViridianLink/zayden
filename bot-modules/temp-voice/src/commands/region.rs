use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};

use crate::{TempVoiceError, VoiceChannelRow, actions};

pub(super) async fn region(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    let region = match options.remove("region") {
        Some(ResolvedValue::String(region)) => Some(region.to_string()),
        _ => None,
    };

    let msg =
        actions::region(http, channel_id, row, interaction.user.id, region).await?;

    interaction
        .edit_response(http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
