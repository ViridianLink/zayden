use std::collections::HashMap;

use serenity::all::{
    ChannelId, CommandInteraction, EditChannel, EditInteractionResponse, Http, ResolvedValue,
};

use crate::error::PermissionError;
use crate::{Error, VoiceChannelRow};

pub async fn bitrate(
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

    let kbps = match options.remove("kbps") {
        Some(ResolvedValue::Integer(kbps)) => kbps as u32,
        _ => unreachable!("Kbps option is required"),
    };

    channel_id
        .edit(http, EditChannel::new().bitrate(kbps * 1000))
        .await
        .unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Channel bitrate updated."),
        )
        .await
        .unwrap();

    Ok(())
}
