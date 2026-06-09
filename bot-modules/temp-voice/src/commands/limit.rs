use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditChannel,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};
use serenity::nonmax::NonMaxU16;

use crate::error::PermissionError;
use crate::{TempVoiceError, VoiceChannelRow};

pub(super) async fn limit(
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

    let limit = match options.remove("user_limit") {
        Some(ResolvedValue::Integer(limit)) => {
            u16::try_from(limit.clamp(0, 99)).unwrap_or(0)
        },
        _ => 0,
    };

    channel_id
        .edit(
            http,
            EditChannel::new()
                .user_limit(NonMaxU16::new(limit).unwrap_or(NonMaxU16::ZERO)),
        )
        .await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new()
                .content(format!("User limit set to {limit}")),
        )
        .await?;

    Ok(())
}
