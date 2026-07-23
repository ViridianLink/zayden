use serenity::all::{CommandInteraction, Context, CreateCommand, Permissions};
use sqlx::PgPool;

use crate::{FamilyError, FamilyRow, Result};

pub struct ResetFamily;

impl ResetFamily {
    pub async fn run(
        _ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        FamilyRow::reset(pool, guild_id).await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("resetfamily")
            .description("Resets the family tree(s) in guild")
            .default_member_permissions(Permissions::ADMINISTRATOR)
    }
}
