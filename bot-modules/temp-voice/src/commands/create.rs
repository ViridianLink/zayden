use std::collections::HashMap;

use serenity::all::{
    ChannelType,
    CreateChannel,
    DiscordJsonError,
    EditInteractionResponse,
    ErrorResponse,
    GuildId,
    Http,
    HttpError,
    JsonErrorCode,
    PermissionOverwrite,
    PermissionOverwriteType,
    Permissions,
    ResolvedValue,
};
use serenity::nonmax::NonMaxU16;
use sqlx::PgPool;
use tracing::debug;

use crate::{
    TempVoiceError,
    TempVoiceRow,
    VoiceChannelRow,
    delete_voice_channel_if_inactive,
    owner_perms,
};

pub(super) async fn create(
    http: &Http,
    interaction: &serenity::all::CommandInteraction,
    pool: &PgPool,
    guild_id: GuildId,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    let name = match options.remove("name") {
        Some(ResolvedValue::String(name)) => name.to_string(),
        _ => format!("{}'s Channel", interaction.user.name),
    };

    let limit = match options.remove("limit") {
        Some(ResolvedValue::Integer(limit)) => {
            u16::try_from(limit.clamp(0, 99)).unwrap_or(0)
        },
        _ => 0,
    };

    let privacy = match options.remove("privacy") {
        Some(ResolvedValue::String(privacy)) => privacy,
        _ => "visible",
    };

    let mut perms = vec![owner_perms(interaction.user.id)];

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
        _ => return Err(TempVoiceError::IneligibleChannel),
    }

    let category = TempVoiceRow::get_category(pool, guild_id).await?;

    let vc_builder = CreateChannel::new(name)
        .kind(ChannelType::Voice)
        .category(category)
        .user_limit(NonMaxU16::new(limit).unwrap_or(NonMaxU16::ZERO))
        .permissions(perms);

    let vc: serenity::all::GuildChannel =
        guild_id.create_channel(http, vc_builder).await?;

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
        .await?;

    // Target user is not connected to voice.
    if let Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
        ErrorResponse {
            error:
                DiscordJsonError {
                    code: JsonErrorCode::TargetUserNotConnectedToVoice,
                    ..
                },
            ..
        },
    ))) = move_result
        && delete_voice_channel_if_inactive(http, guild_id, interaction.user.id, &vc)
            .await
    {
        debug!(user_id = %interaction.user.id, channel_id = %vc.id, "created channel deleted: user did not join within timeout");
        return Ok(());
    }

    let row = VoiceChannelRow::new(vc.id, interaction.user.id);
    row.save(pool).await?;

    Ok(())
}
