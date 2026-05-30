use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditChannel,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};
use serenity::nonmax::NonMaxU16;

use crate::error::PermissionError;
use crate::{Error, VoiceChannelRow};

pub(super) async fn limit(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
) -> Result<(), Error> {
    interaction.defer_ephemeral(http).await?;

    if !row.is_trusted(interaction.user.id) {
        return Err(Error::MissingPermissions(PermissionError::NotTrusted));
    }

    let limit = match options.remove("user_limit") {
        Some(ResolvedValue::Integer(limit)) => {
            u16::try_from(limit.clamp(0, 99)).expect("clamped 0-99 fits in u16")
        },
        _ => 0,
    };

    channel_id
        .edit(
            http,
            EditChannel::new().user_limit(
                NonMaxU16::new(limit).expect("limit 0-99 is below NonMaxU16 max"),
            ),
        )
        .await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new()
                .content(format!("User limit set to {limit}")),
        )
        .await?;

    Ok(())
}
