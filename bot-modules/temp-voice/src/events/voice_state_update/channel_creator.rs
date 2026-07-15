use serenity::all::{
    ChannelType,
    CreateChannel,
    CreateComponent,
    CreateMessage,
    DiscordJsonError,
    ErrorResponse,
    Http,
    HttpError,
    JsonErrorCode,
    VoiceState,
};
use sqlx::{Database, Pool};
use tracing::debug;

use crate::components::build_panel;
use crate::{
    Result,
    TempVoiceError,
    TempVoiceGuildManager,
    VoiceChannelManager,
    VoiceChannelRow,
    delete_voice_channel_if_inactive,
    owner_perms,
};

pub async fn channel_creator<
    Db: Database,
    GuildManager: TempVoiceGuildManager<Db>,
    ChannelManager: VoiceChannelManager<Db>,
>(
    http: &Http,
    pool: &Pool<Db>,
    new: &VoiceState,
) -> Result<()> {
    let Some(guild_id) = new.guild_id else {
        return Err(TempVoiceError::Internal(format!(
            "voice state for user {} has no guild_id",
            new.user_id
        )));
    };

    let creator_channel =
        match GuildManager::get_creator_channel(pool, guild_id).await {
            Ok(Some(channel)) => channel,
            Ok(None) | Err(sqlx::Error::RowNotFound) => {
                debug!(%guild_id, "no creator channel configured for guild");
                return Ok(());
            },
            Err(e) => return Err(e.into()),
        };

    let creator_channel_id = match new.channel_id {
        Some(channel) if channel == creator_channel => channel,
        _ => {
            debug!(user_id = %new.user_id, channel_id = ?new.channel_id, "user joined a channel other than the creator channel");
            return Ok(());
        },
    };

    let creator_category = match creator_channel_id
        .to_guild_channel(http, new.guild_id)
        .await
        .map(|c| c.parent_id)
    {
        Ok(Some(parent_id)) => parent_id,
        Ok(None) => {
            return Err(TempVoiceError::Internal(format!(
                "creator channel {creator_channel_id} has no parent category"
            )));
        },
        Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
            ErrorResponse {
                error: DiscordJsonError { code: JsonErrorCode::MissingAccess, .. },
                ..
            },
        ))) => {
            debug!(
                %creator_channel_id,
                "missing access to creator channel's category; skipping channel creation"
            );
            return Ok(());
        },
        Err(e) => return Err(e.into()),
    };

    let Some(member) = new.member.as_ref() else {
        return Err(TempVoiceError::Internal(format!(
            "voice state for user {} in guild {guild_id} has no member field",
            new.user_id
        )));
    };

    let perms = vec![owner_perms(member.user.id)];

    let vc_builder =
        CreateChannel::new(format!("{}'s Channel", member.display_name()))
            .kind(ChannelType::Voice)
            .category(creator_category)
            .permissions(perms);

    let vc = guild_id.create_channel(http, vc_builder).await?;

    match guild_id.move_member(http, member.user.id, vc.id).await {
        Ok(_) => {},
        Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
            ErrorResponse {
                error:
                    DiscordJsonError {
                        code: JsonErrorCode::TargetUserNotConnectedToVoice,
                        ..
                    },
                ..
            },
        ))) => {
            member
                .user
                .id
                .direct_message(
                    http,
                    CreateMessage::new().content(
                        "Voice channel created. You have 1 minute to join.",
                    ),
                )
                .await?;

            if delete_voice_channel_if_inactive(http, guild_id, member.user.id, &vc)
                .await
            {
                debug!(
                    channel_id = %vc.id,
                    user_id = %member.user.id,
                    "deleted temp voice channel; user did not join within the timeout"
                );
                return Ok(());
            }
        },
        Err(e) => return Err(e.into()),
    }

    let row = VoiceChannelRow::new(vc.id, new.user_id);
    row.save::<Db, ChannelManager>(pool).await?;

    let components = build_panel()
        .into_iter()
        .map(CreateComponent::ActionRow)
        .collect::<Vec<_>>();

    vc.id
        .widen()
        .send_message(
            http,
            CreateMessage::new()
                .content(
                    "**Voice channel controls** - use these buttons to manage your channel.",
                )
                .components(components),
        )
        .await?;

    Ok(())
}
