use std::collections::HashMap;

use serenity::all::{
    ChannelId, CommandInteraction, EditInteractionResponse, Http, PermissionOverwrite,
    PermissionOverwriteType, Permissions, ResolvedValue,
};
use serenity::all::{CreateMessage, Mentionable};

use crate::{Error, VoiceChannelRow};

pub async fn invite(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<(), Error> {
    interaction.defer_ephemeral(http).await.unwrap();

    let user = match options.remove("user") {
        Some(ResolvedValue::User(user, _member)) => user,
        _ => unreachable!("User option is required"),
    };

    row.create_invite(user.id);

    channel_id
        .create_permission(
            http,
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(user.id),
            },
            Some("User invited to channel"),
        )
        .await
        .unwrap();

    let result = user
        .id
        .direct_message(
            http,
            CreateMessage::new().content(format!(
                "You have been invited to {}.",
                channel_id.mention()
            )),
        )
        .await;

    let content = match result {
        Ok(_) => "Sent invite to user.",
        Err(_) => "Invited user, but failed to send DM.",
    };

    interaction
        .edit_response(http, EditInteractionResponse::new().content(content))
        .await
        .unwrap();

    Ok(())
}
