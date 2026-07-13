use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};

use crate::{TempVoiceError, VoiceChannelRow, actions};

pub(super) async fn limit(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    let limit = match options.remove("user_limit") {
        Some(ResolvedValue::Integer(limit)) => limit,
        _ => 0,
    };

    let msg =
        actions::limit(http, channel_id, row, interaction.user.id, limit).await?;

    interaction
        .edit_response(http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
