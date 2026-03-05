use std::collections::HashMap;

use serenity::all::{
    ChannelType, CommandInteraction, CreateChannel, EditInteractionResponse, GuildId, Http,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::{Result, guild_manager::TempVoiceGuildManager};

pub async fn setup<Db: Database, Manager: TempVoiceGuildManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    guild_id: GuildId,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    interaction.defer_ephemeral(http).await.unwrap();

    if !interaction
        .member
        .as_ref()
        .unwrap()
        .permissions
        .unwrap()
        .administrator()
    {
        interaction
            .edit_response(
                http,
                EditInteractionResponse::new()
                    .content("You must be an administrator to run this command."),
            )
            .await
            .unwrap();
        return Ok(());
    }

    let category = match options.remove("category") {
        Some(ResolvedValue::Channel(category)) => category.id().expect_channel(),
        _ => unreachable!("Category is required"),
    };

    let creator_channel = guild_id
        .create_channel(
            http,
            CreateChannel::new("➕ Creator Channel")
                .category(category)
                .kind(ChannelType::Voice),
        )
        .await
        .unwrap();

    Manager::save(pool, guild_id, category, creator_channel.id)
        .await
        .unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content("Setup complete."),
        )
        .await
        .unwrap();

    Ok(())
}
