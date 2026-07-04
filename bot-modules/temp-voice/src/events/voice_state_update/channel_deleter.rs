use serenity::all::{
    Context,
    DiscordJsonError,
    ErrorResponse,
    HttpError,
    JsonErrorCode,
};
use sqlx::{Database, Pool};
use tracing::debug;

use crate::{
    CachedState,
    Result,
    TempVoiceError,
    TempVoiceGuildManager,
    VoiceChannelManager,
    VoiceStateCache,
};

pub async fn channel_deleter<
    Db: Database,
    GuildManager: TempVoiceGuildManager<Db>,
    ChannelManager: VoiceChannelManager<Db>,
>(
    ctx: &Context,
    pool: &Pool<Db>,
    voice_states: &VoiceStateCache,
    old: Option<&CachedState>,
) -> Result<()> {
    let Some(old) = old else {
        debug!("no previous voice state; user joined a channel without leaving one");
        return Ok(());
    };

    let guild_data = match GuildManager::get(pool, old.guild_id).await {
        Ok(row) => row,
        Err(sqlx::Error::RowNotFound) => {
            debug!(guild_id = %old.guild_id, "no temp-voice configuration found for guild");
            return Ok(());
        },
        Err(e) => return Err(e.into()),
    };

    let channel_id = match (old.channel_id, guild_data.creator_channel()) {
        (Some(channel_id), Some(creator_channel))
            if channel_id != creator_channel =>
        {
            channel_id
        },
        (Some(channel_id), Some(creator_channel)) => {
            debug!(%channel_id, %creator_channel, "previous channel was the creator channel; skipping cleanup");
            return Ok(());
        },
        (_, None) => {
            debug!(guild_id = %old.guild_id, "no temp-voice creator channel configured for guild");
            return Ok(());
        },
        (None, Some(creator_channel)) => {
            return Err(TempVoiceError::Internal(format!(
                "voice state cache inconsistency: user {} in guild {} has no previous channel_id but creator channel {creator_channel} is configured",
                old.user_id, old.guild_id
            )));
        },
    };

    let Some(row) = ChannelManager::get(pool, channel_id).await? else {
        debug!(%channel_id, "channel not tracked as a temp-voice channel");
        return Ok(());
    };

    if row.is_persistent() {
        debug!(%channel_id, "persistent channel; user opted out of auto-deletion");
        return Ok(());
    }

    let channel = match channel_id.to_guild_channel(ctx, Some(old.guild_id)).await {
        Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
            ErrorResponse {
                error: DiscordJsonError { code: JsonErrorCode::UnknownChannel, .. },
                ..
            },
        ))) => {
            return Ok(());
        },
        r => r?,
    };

    let Some(category) = guild_data.category() else {
        return Err(TempVoiceError::Internal(format!(
            "guild {} has no temp-voice category configured",
            old.guild_id
        )));
    };

    let Some(parent) = channel.parent_id else {
        return Err(TempVoiceError::Internal(format!(
            "channel {} has no parent category",
            channel.id
        )));
    };
    if parent != category {
        return Err(TempVoiceError::Internal(format!(
            "channel {} is not in the configured temp-voice category {category} (actual parent: {parent})",
            channel.id
        )));
    }

    let users = voice_states.occupants(channel_id).len();

    if users == 0 {
        row.delete::<Db, ChannelManager>(pool).await?;

        match channel_id
            .widen()
            .delete(&ctx.http, Some("Empty temporary voice channel"))
            .await
        {
            // Channel already deleted, ignore this error
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                ErrorResponse {
                    error:
                        DiscordJsonError { code: JsonErrorCode::UnknownChannel, .. },
                    ..
                },
            )))
            | Ok(_) => {},
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}
