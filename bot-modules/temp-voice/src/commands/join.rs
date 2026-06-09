use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    GuildId,
    Http,
    PermissionOverwrite,
    PermissionOverwriteType,
    Permissions,
    ResolvedValue,
};

use crate::{Result, TempVoiceError, VoiceChannelRow};

pub(super) async fn join(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    guild_id: GuildId,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    let Some(ResolvedValue::String(pass)) = options.remove("pass") else {
        return Err(TempVoiceError::IneligibleChannel);
    };

    if !row.verify_password(pass) {
        return Err(TempVoiceError::InvalidPassword);
    }

    channel_id
        .create_permission(
            http,
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(interaction.user.id),
            },
            Some("Correct channel password"),
        )
        .await?;

    guild_id.move_member(http, interaction.user.id, channel_id).await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Successfully joined channel."),
        )
        .await?;

    Ok(())
}
