use std::collections::HashMap;

use serenity::all::{
    ChannelType,
    CommandInteraction,
    CreateChannel,
    EditInteractionResponse,
    GuildId,
    Http,
    Permissions,
    ResolvedValue,
};
use sqlx::PgPool;

use crate::guild_manager::TempVoiceRow;
use crate::{Result, TempVoiceError};

pub(super) async fn setup(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_id: GuildId,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    interaction.defer_ephemeral(http).await?;

    let is_admin = interaction
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .is_some_and(Permissions::administrator);

    if !is_admin {
        return Err(TempVoiceError::AdministratorRequired);
    }

    let Some(ResolvedValue::Channel(category)) = options.remove("category") else {
        return Err(TempVoiceError::IneligibleChannel);
    };
    let category = category.id().expect_channel();

    let creator_channel = guild_id
        .create_channel(
            http,
            CreateChannel::new("➕ Creator Channel")
                .category(category)
                .kind(ChannelType::Voice),
        )
        .await?;

    TempVoiceRow::save(pool, guild_id, category, creator_channel.id).await?;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Setup complete."),
        )
        .await?;

    Ok(())
}
