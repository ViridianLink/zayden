use serenity::all::{ChannelId, EditChannel, Http, UserId};
use serenity::nonmax::NonMaxU16;

use super::require_trusted;
use crate::{Result, TempVoiceError, VoiceChannelRow};

pub async fn rename(
    http: &Http,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
    user_id: UserId,
    name: String,
) -> Result<String> {
    require_trusted(row, user_id)?;

    channel_id.edit(http, EditChannel::new().name(name)).await?;

    Ok("Channel name updated.".to_string())
}

pub async fn limit(
    http: &Http,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
    user_id: UserId,
    limit: i64,
) -> Result<String> {
    require_trusted(row, user_id)?;

    let limit = u16::try_from(limit.clamp(0, 99)).unwrap_or(0);

    channel_id
        .edit(
            http,
            EditChannel::new()
                .user_limit(NonMaxU16::new(limit).unwrap_or(NonMaxU16::ZERO)),
        )
        .await?;

    Ok(format!("User limit set to {limit}"))
}

pub async fn bitrate(
    http: &Http,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
    user_id: UserId,
    kbps: i64,
) -> Result<String> {
    require_trusted(row, user_id)?;

    let kbps = u32::try_from(kbps)
        .map_err(|_kbps_err| TempVoiceError::IneligibleChannel)?;

    channel_id.edit(http, EditChannel::new().bitrate(kbps * 1000)).await?;

    Ok("Channel bitrate updated.".to_string())
}

pub async fn region(
    http: &Http,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
    user_id: UserId,
    region: Option<String>,
) -> Result<String> {
    require_trusted(row, user_id)?;

    channel_id
        .edit(http, EditChannel::new().voice_region(region.map(Into::into)))
        .await?;

    Ok("Channel region updated.".to_string())
}
