use serenity::all::{ChannelId, CommandInteraction, EditInteractionResponse, Http};
use sqlx::PgPool;

use crate::{TempVoiceError, VoiceChannelRow, actions};

pub(super) async fn delete(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &PgPool,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    let msg =
        actions::delete(http, pool, channel_id, row, interaction.user.id).await?;

    interaction
        .edit_response(http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
