use serenity::all::{ChannelId, Http, UserId};
use sqlx::PgPool;

use super::require_owner;
use crate::{
    Result,
    TempVoiceError,
    VoiceChannelRow,
    VoiceStateCache,
    owner_perms,
    revoke_previous_owner,
};

pub async fn claim(
    http: &Http,
    pool: &PgPool,
    voice_states: &VoiceStateCache,
    channel_id: ChannelId,
    row: VoiceChannelRow,
    user_id: UserId,
) -> Result<String> {
    if row.is_owner(user_id) {
        return Err(TempVoiceError::UserIsOwner);
    }

    let owner_present =
        voice_states.current_channel(row.owner_id()) == Some(row.channel_id());

    if !row.is_persistent() && owner_present {
        return Err(TempVoiceError::OwnerInChannel);
    }

    let previous_owner = row.owner_id();

    if !VoiceChannelRow::claim(pool, channel_id, previous_owner, user_id).await? {
        return Err(TempVoiceError::ClaimFailed);
    }

    channel_id
        .create_permission(http, owner_perms(user_id), Some("Channel claimed"))
        .await?;

    if let Some(kind) = revoke_previous_owner(previous_owner, user_id) {
        channel_id.delete_permission(http, kind, Some("Channel claimed")).await?;
    }

    Ok("Claimed channel.".to_string())
}

pub async fn transfer(
    http: &Http,
    pool: &PgPool,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
    user_id: UserId,
    target: UserId,
) -> Result<String> {
    require_owner(&row, user_id)?;

    let previous_owner = row.owner_id();

    row.set_owner(target);
    row.save(pool).await?;

    channel_id
        .create_permission(http, owner_perms(target), Some("Channel transfered"))
        .await?;

    if let Some(kind) = revoke_previous_owner(previous_owner, target) {
        channel_id.delete_permission(http, kind, Some("Channel transfered")).await?;
    }

    Ok("Transferred channel.".to_string())
}

pub async fn delete(
    http: &Http,
    pool: &PgPool,
    channel_id: ChannelId,
    row: VoiceChannelRow,
    user_id: UserId,
) -> Result<String> {
    require_owner(&row, user_id)?;

    row.delete(pool).await?;

    channel_id.widen().delete(http, Some("User deleted channel")).await?;

    Ok("Channel deleted.".to_string())
}
