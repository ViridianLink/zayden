use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::{Result, TempVoiceError, VoiceChannelManager, VoiceChannelRow, actions};

pub(super) async fn transfer<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    let Some(ResolvedValue::User(user, _)) = options.remove("user") else {
        return Err(TempVoiceError::IneligibleChannel);
    };

    let msg = actions::transfer::<Db, Manager>(
        http,
        pool,
        channel_id,
        row,
        interaction.user.id,
        user.id,
    )
    .await?;

    interaction
        .edit_response(http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
