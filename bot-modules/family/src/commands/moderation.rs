use serenity::all::{CommandInteraction, Context, CreateCommand, Permissions};
use sqlx::{Database, Pool};

use crate::{FamilyError, FamilyManager, Result};

pub struct ResetFamily;

impl ResetFamily {
    pub async fn run<Db: Database, Manager: FamilyManager<Db>>(
        _ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        Manager::reset(pool, guild_id).await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("resetfamily")
            .description("Resets the family tree(s) in guild")
            .default_member_permissions(Permissions::ADMINISTRATOR)
    }
}
