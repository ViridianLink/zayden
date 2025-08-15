use serenity::all::{ChannelId, EditInteractionResponse};
use serenity::all::{CommandInteraction, Context};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;

use crate::{Error, VoiceChannelManager, VoiceChannelRow, VoiceStateCache, owner_perms};

pub async fn claim<Data: VoiceStateCache, Db: Database, Manager: VoiceChannelManager<Db>>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    channel_id: ChannelId,
    row: Option<VoiceChannelRow>,
) -> Result<(), Error> {
    interaction.defer_ephemeral(&ctx.http).await.unwrap();

    let mut row = match row {
        Some(row) => {
            if row.is_owner(interaction.user.id) {
                return Err(Error::UserIsOwner);
            }

            row
        }
        None => VoiceChannelRow::new(channel_id, interaction.user.id),
    };

    if !row.is_persistent() && is_claimable::<Data>(ctx, &row).await {
        return Err(Error::OwnerInChannel);
    }

    row.set_owner(interaction.user.id);
    row.save::<Db, Manager>(pool).await?;

    channel_id
        .create_permission(
            &ctx.http,
            owner_perms(interaction.user.id),
            Some("Channel claimed"),
        )
        .await
        .unwrap();

    interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("Claimed channel."),
        )
        .await
        .unwrap();

    Ok(())
}

async fn is_claimable<Data: VoiceStateCache>(
    ctx: &Context,
    channel_data: &VoiceChannelRow,
) -> bool {
    let data = ctx.data::<RwLock<Data>>();
    let data = data.read().await;
    let cache = data.get();

    let owner_state = cache.get(&channel_data.owner_id());

    owner_state.and_then(|state| state.channel_id) == Some(channel_data.channel_id())
}
