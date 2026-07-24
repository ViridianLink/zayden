use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    GenericInteractionChannel,
    Http,
    ResolvedValue,
    Role,
};
use sqlx::PgPool;
use zayden_core::{optional_option, required_option};

use super::Command;
use crate::modals::create::GuildRow;
use crate::{LfgError, Result};

impl Command {
    pub async fn setup(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &PgPool,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let guild_id = interaction.guild_id.ok_or(LfgError::MissingGuildId)?;

        let channel: &GenericInteractionChannel =
            required_option(&mut options, "channel")?;

        let role = optional_option(&mut options, "role").map(|role: &Role| role.id);

        GuildRow::insert(pool, guild_id, channel.id(), role).await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content("LFG plugin has been setup"),
            )
            .await?;

        Ok(())
    }
}
