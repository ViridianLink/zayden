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

pub(super) async fn bitrate(
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

    let Some(ResolvedValue::Integer(kbps)) = options.remove("kbps") else {
        return Err(TempVoiceError::IneligibleChannel);
    };
    let kbps = u32::try_from(kbps)
        .map_err(|_kbps_err| TempVoiceError::IneligibleChannel)?;

    channel_id.edit(http, EditChannel::new().bitrate(kbps * 1000)).await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Channel bitrate updated."),
        )
        .await?;

    Ok(())
}
