use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    PermissionOverwrite,
    PermissionOverwriteType,
    Permissions,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::error::PermissionError;
use crate::{TempVoiceError, VoiceChannelManager, VoiceChannelRow};

pub(super) async fn trust<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    if !row.is_owner(interaction.user.id) {
        return Err(TempVoiceError::MissingPermissions(PermissionError::NotOwner));
    }

    let Some(ResolvedValue::User(user, _member)) = options.remove("user") else {
        return Err(TempVoiceError::IneligibleChannel);
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
        .await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Set user to trusted."),
        )
        .await?;

    Ok(())
}
