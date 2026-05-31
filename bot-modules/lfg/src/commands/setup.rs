use std::collections::HashMap;

use async_trait::async_trait;
use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    GenericChannelId,
    GuildId,
    Http,
    ResolvedValue,
    RoleId,
};
use sqlx::{Database, Pool};

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

        let Some(ResolvedValue::Channel(channel)) = options.remove("channel") else {
            return Ok(());
        };

        let role = match options.remove("role") {
            Some(ResolvedValue::Role(role)) => Some(role.id),
            _ => None,
        };

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
