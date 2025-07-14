use std::collections::HashMap;

use serenity::all::{
    ChannelId, CommandInteraction, EditChannel, EditInteractionResponse, Http, ResolvedValue,
};

use crate::error::PermissionError;
use crate::{Error, VoiceChannelRow};

pub async fn name(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
) -> Result<(), Error> {
    interaction.defer_ephemeral(http).await.unwrap();

    if !row.is_trusted(interaction.user.id) {
        return Err(Error::MissingPermissions(PermissionError::NotTrusted));
    }

    let name = match options.remove("name") {
        Some(ResolvedValue::String(name)) => name.to_string(),
        _ => format!("{}'s Channel", interaction.user.name),
    };

    channel_id
        .edit(http, EditChannel::new().name(name))
        .await
        .unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Channel name updated."),
        )
        .await
        .unwrap();

    Ok(())
}
