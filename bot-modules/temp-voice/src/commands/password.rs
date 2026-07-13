use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    GuildId,
    Http,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::{Result, TempVoiceError, VoiceChannelManager, VoiceChannelRow, actions};

pub(super) async fn password<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    guild_id: GuildId,
    channel_id: ChannelId,
    row: VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    let Some(ResolvedValue::String(pass)) = options.remove("password") else {
        return Err(TempVoiceError::IneligibleChannel);
    };

    let msg = actions::password::<Db, Manager>(
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
