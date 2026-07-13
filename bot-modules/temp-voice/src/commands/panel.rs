use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    CreateComponent,
    CreateMessage,
    EditInteractionResponse,
    Http,
    Permissions,
    ResolvedValue,
};

use crate::components::build_panel;
use crate::{Result, TempVoiceError};

pub(super) async fn panel(
    http: &Http,
    interaction: &CommandInteraction,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    let can_manage = interaction
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .is_some_and(Permissions::manage_channels);

    if !can_manage {
        return Err(TempVoiceError::AdministratorRequired);
    }

    let Some(ResolvedValue::Channel(channel)) = options.remove("channel") else {
        return Err(TempVoiceError::IneligibleChannel);
    };
    let target = channel.id().expect_channel();

    let components = build_panel()
        .into_iter()
        .map(CreateComponent::ActionRow)
        .collect::<Vec<_>>();

    target
        .widen()
        .send_message(
            http,
            CreateMessage::new()
                .content(
                    "**Voice channel controls** — use these buttons to manage the voice channel you're connected to.",
                )
                .components(components),
        )
        .await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Panel posted."),
        )
        .await?;

    Ok(())
}
