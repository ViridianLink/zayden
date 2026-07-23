use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    GuildId,
    Http,
    ResolvedValue,
};
use sqlx::PgPool;

use crate::{Result, TempVoiceError, VoiceChannelRow, actions};

pub(super) async fn password(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &PgPool,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    guild_id: GuildId,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    let Some(ResolvedValue::String(pass)) = options.remove("password") else {
        return Err(TempVoiceError::IneligibleChannel);
    };

    let msg = actions::password(
        http,
        pool,
        guild_id,
        channel_id,
        row,
        interaction.user.id,
        pass.to_string(),
    )
    .await?;

    interaction
        .edit_response(http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}
