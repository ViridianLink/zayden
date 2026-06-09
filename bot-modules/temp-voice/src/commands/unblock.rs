use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    PermissionOverwriteType,
    ResolvedValue,
};

use crate::error::PermissionError;
use crate::{TempVoiceError, VoiceChannelRow};

pub(super) async fn unblock(
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

    let Some(ResolvedValue::User(user, _member)) = options.remove("user") else {
        return Err(TempVoiceError::IneligibleChannel);
    };

    channel_id
        .delete_permission(
            http,
            PermissionOverwriteType::Member(user.id),
            Some("User unblocked"),
        )
        .await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Removed user from blocked."),
        )
        .await?;

    Ok(())
}
