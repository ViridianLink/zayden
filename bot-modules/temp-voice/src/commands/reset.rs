use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditChannel,
    EditInteractionResponse,
    GuildId,
    Http,
    PermissionOverwriteType,
};
use serenity::nonmax::NonMaxU16;
use sqlx::PgPool;

use crate::error::PermissionError;
use crate::{Result, TempVoiceError, VoiceChannelRow};

pub(super) async fn reset(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_id: GuildId,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    if !row.is_owner(interaction.user.id) {
        return Err(TempVoiceError::MissingPermissions(PermissionError::NotOwner));
    }

    row.reset();
    row.save(pool).await?;

    let channel = guild_id
        .channels(http)
        .await?
        .remove(&channel_id)
        .ok_or(TempVoiceError::ChannelNotFound(channel_id))?;

    let everyone_permissions = channel
        .permission_overwrites
        .iter()
        .find(|perm| {
            perm.kind == PermissionOverwriteType::Role(guild_id.everyone_role())
        })
        .ok_or(TempVoiceError::IneligibleChannel)?;

    channel_id
        .edit(
            http,
            EditChannel::new()
                .name(format!("{}'s Channel", interaction.user.display_name()))
                .user_limit(NonMaxU16::ZERO)
                .permissions(vec![everyone_permissions.clone()]),
        )
        .await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Reset channel."),
        )
        .await?;

    Ok(())
}
