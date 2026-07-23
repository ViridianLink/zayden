use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};
use sqlx::PgPool;

use crate::{Result, TempVoiceError, VoiceChannelRow, actions};

pub(super) async fn transfer(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &PgPool,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    let Some(ResolvedValue::User(user, _)) = options.remove("user") else {
        return Err(TempVoiceError::IneligibleChannel);
    };

    let msg =
        actions::transfer(http, pool, channel_id, row, interaction.user.id, user.id)
            .await?;

    interaction
        .edit_response(http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
