use serenity::all::{
    ChannelId,
    CommandInteraction,
    Context,
    EditInteractionResponse,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;

use crate::{
    TempVoiceError,
    VoiceChannelManager,
    VoiceChannelRow,
    VoiceStateCache,
    owner_perms,
};

pub(super) async fn claim<
    Data: VoiceStateCache,
    Db: Database,
    Manager: VoiceChannelManager<Db>,
>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(&ctx.http).await?;

    if row.is_owner(interaction.user.id) {
        return Err(TempVoiceError::UserIsOwner);
    }

    if !row.is_persistent() && is_claimable::<Data>(ctx, &row).await {
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

async fn is_claimable<Data: VoiceStateCache>(
    ctx: &Context,
    channel_data: &VoiceChannelRow,
) -> bool {
    let data = ctx.data::<RwLock<Data>>();
    let guard = data.read().await;
    let result =
        guard.get().get(&channel_data.owner_id()).and_then(|state| state.channel_id)
            == Some(channel_data.channel_id());
    drop(guard);
    result
}
