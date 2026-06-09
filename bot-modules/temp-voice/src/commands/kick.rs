use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    GuildId,
    Http,
    ResolvedValue,
};

use crate::error::PermissionError;
use crate::{TempVoiceError, VoiceChannelRow};

pub(super) async fn kick(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    guild_id: GuildId,
    row: &VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    if !row.is_trusted(interaction.user.id) {
        return Err(TempVoiceError::MissingPermissions(PermissionError::NotTrusted));
    }

    let Some(ResolvedValue::User(user, _)) = options.remove("member") else {
        return Err(TempVoiceError::IneligibleChannel);
    };

    guild_id.disconnect_member(http, user.id).await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("User kicked from channel."),
        )
        .await?;

    Ok(())
}
