use serenity::all::{ChannelId, CommandInteraction, EditInteractionResponse, Http};
use sqlx::{Database, Pool};

use crate::error::PermissionError;
use crate::{Error, VoiceChannelManager, VoiceChannelRow};

pub async fn delete<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<(), Error> {
    interaction.defer_ephemeral(http).await.unwrap();

    if row.is_owner(interaction.user.id) {
        return Err(Error::MissingPermissions(PermissionError::NotOwner));
    }

    row.delete::<Db, Manager>(pool).await?;

    channel_id
        .widen()
        .delete(http, Some("User deleted channel"))
        .await
        .unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Channel deleted."),
        )
        .await
        .unwrap();

    Ok(())
}
