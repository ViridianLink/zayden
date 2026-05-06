use std::collections::HashMap;

use serenity::all::{CommandInteraction, EditInteractionResponse, GuildId, Http, ResolvedValue};

use crate::error::PermissionError;
use crate::{Error, VoiceChannelRow};

pub async fn kick(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    guild_id: GuildId,
    row: &VoiceChannelRow,
) -> Result<(), Error> {
    interaction.defer_ephemeral(http).await.unwrap();

    if !row.is_trusted(interaction.user.id) {
        return Err(Error::MissingPermissions(PermissionError::NotTrusted));
    }

    let user = match options.remove("member") {
        Some(ResolvedValue::User(user, _)) => user,
        _ => unreachable!("Member option is required"),
    };

    guild_id.disconnect_member(http, user.id).await.unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("User kicked from channel."),
        )
        .await
        .unwrap();

    Ok(())
}
