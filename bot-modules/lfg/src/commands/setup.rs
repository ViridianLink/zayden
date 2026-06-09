use std::collections::HashMap;

use async_trait::async_trait;
use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    GenericChannelId,
    GenericInteractionChannel,
    GuildId,
    Http,
    ResolvedValue,
    Role,
    RoleId,
};
use sqlx::{Database, Pool};
use zayden_core::{optional_option, required_option};

use super::Command;
use crate::{LfgError, Result};

#[async_trait]
pub trait SetupManager<Db: Database> {
    async fn insert(
        pool: &Pool<Db>,
        id: impl Into<GuildId> + Send,
        channel: impl Into<GenericChannelId> + Send,
        role: Option<RoleId>,
    ) -> sqlx::Result<Db::QueryResult>;
}

impl Command {
    pub async fn setup<Db: Database, Manager: SetupManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let guild_id = interaction.guild_id.ok_or(LfgError::MissingGuildId)?;

        let channel: &GenericInteractionChannel =
            required_option(&mut options, "channel")?;

        let role = optional_option(&mut options, "role").map(|role: &Role| role.id);

        Manager::insert(pool, guild_id, channel.id(), role).await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content("LFG plugin has been setup"),
            )
            .await?;

        Ok(())
    }
}
