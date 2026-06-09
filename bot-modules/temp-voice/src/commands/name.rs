use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditChannel,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};

use crate::error::PermissionError;
use crate::{TempVoiceError, VoiceChannelRow};

pub(super) async fn name(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    if !row.is_trusted(interaction.user.id) {
        return Err(TempVoiceError::MissingPermissions(PermissionError::NotTrusted));
    }

    let name = match options.remove("name") {
        Some(ResolvedValue::String(name)) => name.to_string(),
        _ => format!("{}'s Channel", interaction.user.name),
    };

    channel_id.edit(http, EditChannel::new().name(name)).await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Channel name updated."),
        )
        .await?;

    Ok(())
}
