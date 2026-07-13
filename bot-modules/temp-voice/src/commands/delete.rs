use serenity::all::{ChannelId, CommandInteraction, EditInteractionResponse, Http};
use sqlx::{Database, Pool};

use crate::{TempVoiceError, VoiceChannelManager, VoiceChannelRow, actions};

pub(super) async fn delete<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    let msg = actions::delete::<Db, Manager>(
        http,
        pool,
        channel_id,
        row,
        interaction.user.id,
    )
    .await?;

    interaction
        .edit_response(http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
