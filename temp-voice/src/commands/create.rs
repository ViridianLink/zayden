use std::collections::HashMap;

use serenity::all::{
    ChannelType, CreateChannel, GuildId, Http, JsonErrorCode, PermissionOverwrite,
    PermissionOverwriteType, Permissions, ResolvedValue,
};
use serenity::all::{DiscordJsonError, EditInteractionResponse, ErrorResponse, HttpError};
use serenity::nonmax::NonMaxU16;
use sqlx::{Database, Pool};

use crate::{
    Error, TempVoiceGuildManager, VoiceChannelManager, VoiceChannelRow,
    delete_voice_channel_if_inactive,
};

pub async fn create<
    Db: Database,
    GuildManager: TempVoiceGuildManager<Db>,
    ChannelManager: VoiceChannelManager<Db>,
>(
    http: &Http,
    interaction: &serenity::all::CommandInteraction,
    pool: &Pool<Db>,
    guild_id: GuildId,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<(), Error> {
    interaction.defer_ephemeral(http).await.unwrap();

    let name = match options.remove("name") {
        Some(ResolvedValue::String(name)) => name.to_string(),
        _ => format!("{}'s Channel", interaction.user.name),
    };

    let limit = match options.remove("limit") {
        Some(ResolvedValue::Integer(limit)) => limit.clamp(0, 99) as u16,
        _ => 0,
    };

    let privacy = match options.remove("privacy") {
        Some(ResolvedValue::String(privacy)) => privacy,
        _ => "visible",
    };

    let mut perms = vec![PermissionOverwrite {
        allow: Permissions::all(),
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(interaction.user.id),
    }];

    match privacy {
        "lock" => perms.push(PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::CONNECT,
            kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
        }),
        "unlock" => perms.push(PermissionOverwrite {
            allow: Permissions::CONNECT,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
        }),
        "invisible" => perms.push(PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
        }),
        "visible" => perms.push(PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(guild_id.everyone_role()),
        }),
        _ => unreachable!("Invalid privacy option"),
    };

    let category = GuildManager::get_category(pool, guild_id).await.unwrap();

    let vc_builder = CreateChannel::new(name)
        .kind(ChannelType::Voice)
        .category(category)
        .user_limit(NonMaxU16::new(limit).unwrap())
        .permissions(perms);

    let vc: serenity::all::GuildChannel = guild_id.create_channel(http, vc_builder).await.unwrap();

    let move_result = guild_id.move_member(http, interaction.user.id, vc.id).await;

    let response_content = match move_result {
        Ok(_) => "Voice channel created and you have been moved successfully.",
        Err(_) => "Voice channel created. You have 1 minute to join.",
    };

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content(response_content),
        )
        .await
        .unwrap();

    // Target user is not connected to voice.
    if let Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
        error:
            DiscordJsonError {
                code: JsonErrorCode::TargetUserNotConnectedToVoice,
                ..
            },
        ..
    }))) = move_result
        && delete_voice_channel_if_inactive(http, guild_id, interaction.user.id, &vc).await
    {
        return Ok(());
    }

    let row = VoiceChannelRow::new(vc.id, interaction.user.id);
    row.save::<Db, ChannelManager>(pool).await?;

    Ok(())
}
