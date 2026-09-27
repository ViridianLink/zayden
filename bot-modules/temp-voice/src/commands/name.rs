use std::collections::HashMap;

use serenity::all::{
    ChannelId,
    CommandInteraction,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};
use zayden_core::optional_option;

use crate::{TempVoiceError, VoiceChannelRow, actions};

pub(super) async fn name(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
    channel_id: ChannelId,
    row: &VoiceChannelRow,
) -> Result<(), TempVoiceError> {
    interaction.defer_ephemeral(http).await?;

    let name =
        channel_name(optional_option(&mut options, "name"), &interaction.user.name);

    let msg =
        actions::rename(http, channel_id, row, interaction.user.id, name).await?;

    interaction
        .edit_response(http, EditInteractionResponse::new().content(msg))
        .await?;

    Ok(())
}

pub(super) fn channel_name(raw: Option<&str>, username: &str) -> String {
    let name = raw.map_or_else(String::new, |name| {
        name.split_whitespace().collect::<Vec<_>>().join(" ")
    });

    if name.is_empty() { format!("{username}'s Channel") } else { name }
}
