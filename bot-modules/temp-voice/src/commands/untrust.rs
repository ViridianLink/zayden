use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    PermissionOverwriteType,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::error::PermissionError;
use crate::{Error, VoiceChannelManager, VoiceChannelRow};

pub(super) async fn untrust<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<(), Error> {
    interaction.defer_ephemeral(http).await?;

    if !row.is_owner(interaction.user.id) {
        return Err(Error::MissingPermissions(PermissionError::NotOwner));
    }

    let Some(ResolvedValue::User(user, _member)) = options.remove("user") else {
        return Err(Error::IneligibleChannel);
    };

    row.untrust(user.id);
    row.save::<Db, Manager>(pool).await?;

    channel_id
        .delete_permission(
            http,
            PermissionOverwriteType::Member(user.id),
            Some("User untrusted"),
        )
        .await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Removed user from trusted."),
        )
        .await?;

    Ok(())
}
