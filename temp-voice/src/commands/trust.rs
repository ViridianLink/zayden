use std::collections::HashMap;

use serenity::all::{ChannelId, EditInteractionResponse, Http};
use serenity::all::{
    CommandInteraction, PermissionOverwrite, PermissionOverwriteType, Permissions, ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::error::PermissionError;
use crate::{Error, VoiceChannelManager, VoiceChannelRow};

pub async fn trust<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<(), Error> {
    interaction.defer_ephemeral(http).await.unwrap();

    if !row.is_owner(interaction.user.id) {
        return Err(Error::MissingPermissions(PermissionError::NotOwner));
    }

    let user = match options.remove("user") {
        Some(ResolvedValue::User(user, _member)) => user,
        _ => unreachable!("User option is required"),
    };

    row.trust(user.id);
    row.save::<Db, Manager>(pool).await?;

    channel_id
        .create_permission(
            http,
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::MANAGE_CHANNELS
                    | Permissions::CONNECT
                    | Permissions::SET_VOICE_CHANNEL_STATUS,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(user.id),
            },
            Some("User trusted"),
        )
        .await
        .unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Set user to trusted."),
        )
        .await
        .unwrap();

    Ok(())
}
