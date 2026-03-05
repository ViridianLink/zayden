use std::collections::HashMap;

use serenity::all::{
    ChannelId, CommandInteraction, GuildId, Http, PermissionOverwrite, PermissionOverwriteType,
    Permissions,
};
use serenity::all::{EditInteractionResponse, ResolvedValue};

use crate::{Error, Result, VoiceChannelRow};

pub async fn join(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    guild_id: GuildId,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await.unwrap();

    let pass = match options.remove("pass") {
        Some(ResolvedValue::String(pass)) => pass,
        _ => unreachable!("Password option is required"),
    };

    if !row.verify_password(pass) {
        return Err(Error::InvalidPassword);
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
        .await
        .unwrap();

    guild_id
        .move_member(http, interaction.user.id, channel_id)
        .await
        .unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Successfully joined channel."),
        )
        .await
        .unwrap();

    Ok(())
}
