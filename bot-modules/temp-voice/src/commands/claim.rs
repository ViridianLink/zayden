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
    owner_perms,
};

pub(super) async fn claim<Db: Database, Manager: VoiceChannelManager<Db>>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    voice_states: &VoiceStateCache,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(&ctx.http).await?;

    if row.is_owner(interaction.user.id) {
        return Err(TempVoiceError::UserIsOwner);
    }

    if !row.is_persistent() && is_claimable(voice_states, &row) {
        return Err(TempVoiceError::OwnerInChannel);
    }

    row.set_owner(interaction.user.id);
    row.save::<Db, Manager>(pool).await?;

    channel_id
        .create_permission(
            &ctx.http,
            owner_perms(interaction.user.id),
            Some("Channel claimed"),
        )
        .await?;

    interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("Claimed channel."),
        )
        .await?;

    Ok(())
}

fn is_claimable(
    voice_states: &VoiceStateCache,
    channel_data: &VoiceChannelRow,
) -> bool {
    voice_states.current_channel(channel_data.owner_id())
        == Some(channel_data.channel_id())
}
