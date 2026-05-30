use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::error::PermissionError;
use crate::{Error, Result, VoiceChannelManager, VoiceChannelRow, owner_perms};

pub(super) async fn transfer<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    if !row.is_owner(interaction.user.id) {
        return Err(Error::MissingPermissions(PermissionError::NotOwner));
    }

    let Some(ResolvedValue::User(user, _)) = options.remove("user") else {
        return Err(Error::IneligibleChannel);
    };

    row.set_owner(user.id);
    row.save::<Db, Manager>(pool).await?;

    channel_id
        .create_permission(http, owner_perms(user.id), Some("Channel transfered"))
        .await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Transferred channel."),
        )
        .await?;

    Ok(())
}
