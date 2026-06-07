use serenity::all::{
    Context,
    DiscordJsonError,
    ErrorResponse,
    HttpError,
    JsonErrorCode,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;

use crate::{
    CachedState,
    Result,
    TempVoiceGuildManager,
    VoiceChannelManager,
    VoiceStateCache,
};

pub async fn channel_deleter<
    Data: VoiceStateCache,
    Db: Database,
    GuildManager: TempVoiceGuildManager<Db>,
    ChannelManager: VoiceChannelManager<Db>,
>(
    ctx: &Context,
    pool: &Pool<Db>,
    old: Option<&CachedState>,
) -> Result<()> {
    let Some(old) = old else {
        return Ok(());
    };

    let Ok(guild_data) = GuildManager::get(pool, old.guild_id).await else {
        return Ok(());
    };

    let channel_id = match (old.channel_id, guild_data.creator_channel()) {
        (Some(channel_id), Some(creator_channel))
            if channel_id != creator_channel =>
        {
            channel_id
        },
        _ => return Ok(()),
    };

    let Some(row) = ChannelManager::get(pool, channel_id).await? else {
        return Ok(());
    };

    if row.is_persistent() {
        return Ok(());
    }

    let channel = match channel_id.to_guild_channel(ctx, Some(old.guild_id)).await {
        Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
            ErrorResponse {
                error:
                    DiscordJsonError {
                        code:
                            JsonErrorCode::UnknownChannel | JsonErrorCode::MissingAccess,
                        ..
                    },
                ..
            },
        ))) => {
            return Ok(());
        },
        r => r?,
    };
    let Some(category) = guild_data.category() else {
        return Ok(());
    };

    let Some(parent) = channel.parent_id else {
        return Ok(());
    };
    if parent != category {
        return Ok(());
    }

    let users = {
        let data = ctx.data::<RwLock<Data>>();
        let guard = data.read().await;
        let count = guard
            .get()
            .values()
            .filter(|id| id.channel_id == Some(channel_id))
            .count();
        drop(guard);
        count
    };

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
