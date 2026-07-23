use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    PermissionOverwriteType,
    ResolvedValue,
};
use sqlx::PgPool;

use crate::error::PermissionError;
use crate::{TempVoiceError, VoiceChannelRow};

pub(super) async fn untrust(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &PgPool,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    mut row: VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    if !row.is_owner(interaction.user.id) {
        return Err(TempVoiceError::MissingPermissions(PermissionError::NotOwner));
    }

    let Some(ResolvedValue::User(user, _member)) = options.remove("user") else {
        return Err(TempVoiceError::IneligibleChannel);
    };

    row.untrust(user.id);
    row.save(pool).await?;

    channel_id
        .delete_permission(
            http,
            PermissionOverwriteType::Member(user.id),
            Some("User untrusted"),
        )
        .await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Removed user from trusted."),
        )
        .await?;

    Ok(())
}
