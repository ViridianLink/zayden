use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    GuildId,
    Http,
    ResolvedValue,
};

use crate::{TempVoiceError, VoiceChannelRow, actions};

pub(super) async fn kick(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    guild_id: GuildId,
    row: &VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    let Some(ResolvedValue::User(user, _)) = options.remove("member") else {
        return Err(TempVoiceError::IneligibleChannel);
    };

    let msg =
        actions::kick(http, guild_id, row, interaction.user.id, user.id).await?;

    interaction
        .edit_response(http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
