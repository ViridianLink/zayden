use serenity::all::{
    ChannelId,
    CommandInteraction,
    Context,
    EditInteractionResponse,
};
use sqlx::{Database, Pool};

use crate::{
    TempVoiceError,
    VoiceChannelManager,
    VoiceChannelRow,
    VoiceStateCache,
    actions,
};

pub(super) async fn claim<Db: Database, Manager: VoiceChannelManager<Db>>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    voice_states: &VoiceStateCache,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(&ctx.http).await?;

    let msg = actions::claim::<Db, Manager>(
        &ctx.http,
        pool,
        voice_states,
        channel_id,
        row,
        interaction.user.id,
    )
    .await?;

    interaction
        .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
