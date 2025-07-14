use std::collections::HashMap;

use serenity::all::{
    ChannelId, CommandInteraction, EditChannel, EditInteractionResponse, Http, ResolvedValue,
};

use crate::error::PermissionError;
use crate::{Error, VoiceChannelRow};

pub async fn region(
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

    let region = match options.remove("region") {
        Some(ResolvedValue::String(region)) => Some(region),
        _ => None,
    };

    channel_id
        .edit(
            http,
            EditChannel::new().voice_region(region.map(|r| r.into())),
        )
        .await
        .unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Channel region updated."),
        )
        .await
        .unwrap();

    Ok(())
}
