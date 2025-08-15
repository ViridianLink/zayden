use serenity::all::{
    ChannelType, CreateChannel, CreateMessage, DiscordJsonError, ErrorResponse, Http, HttpError,
    JsonErrorCode, VoiceState,
};
use sqlx::{Database, Pool};

use crate::{
    Result, TempVoiceGuildManager, VoiceChannelManager, VoiceChannelRow,
    delete_voice_channel_if_inactive, owner_perms,
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
    let guild_id = new
        .guild_id
        .expect("Should be in a guild as voice channels are guild only");

    let Ok(Some(creator_channel)) = GuildManager::get_creator_channel(pool, guild_id).await else {
        return Ok(());
    };

    let creator_channel_id = match new.channel_id {
        Some(channel) if channel == creator_channel => channel,
        _ => return Ok(()),
    };

    let creator_category = creator_channel_id
        .to_guild_channel(http, new.guild_id)
        .await
        .unwrap()
        .parent_id
        .expect("Should be in a category");

    let member = new.member.as_ref().expect("Should be in a guild");

    let perms = vec![owner_perms(member.user.id)];

    let vc_builder = CreateChannel::new(format!("{}'s Channel", member.display_name()))
        .kind(ChannelType::Voice)
        .category(creator_category)
        .permissions(perms);

    let vc = guild_id.create_channel(http, vc_builder).await?;

    match guild_id.move_member(http, member.user.id, vc.id).await {
        // Target user is not connected to voice.
        Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
            error:
                DiscordJsonError {
                    code: JsonErrorCode::TargetUserNotConnectedToVoice,
                    ..
                },
            ..
        }))) => {
            member
                .user
                .id
                .direct_message(
                    http,
                    CreateMessage::new()
                        .content("Voice channel created. You have 1 minute to join."),
                )
                .await
                .unwrap();

            if delete_voice_channel_if_inactive(http, guild_id, member.user.id, &vc).await {
                return Ok(());
            }
        }
        result => {
            result.unwrap();
        }
    };

    let row = VoiceChannelRow::new(vc.id, new.user_id);
    row.save::<Db, ChannelManager>(pool).await?;

    Ok(())
}
