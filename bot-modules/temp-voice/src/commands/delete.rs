use serenity::all::{ChannelId, CommandInteraction, EditInteractionResponse, Http};
use sqlx::{Database, Pool};

use crate::error::PermissionError;
use crate::{TempVoiceError, VoiceChannelManager, VoiceChannelRow};

pub(super) async fn delete<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    if row.is_owner(interaction.user.id) {
        return Err(TempVoiceError::MissingPermissions(PermissionError::NotOwner));
    }

    row.delete::<Db, Manager>(pool).await?;

    channel_id.widen().delete(http, Some("User deleted channel")).await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Channel deleted."),
        )
        .await?;

    Ok(())
}
