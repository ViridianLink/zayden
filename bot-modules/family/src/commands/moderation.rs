use serenity::all::{CommandInteraction, Context, CreateCommand, Permissions};
use sqlx::{Database, Pool};

use crate::{FamilyManager, Result};

pub struct ResetFamily;

impl ResetFamily {
    pub async fn run<Db: Database, Manager: FamilyManager<Db>>(
        _ctx: &Context,
        _interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        Manager::reset(pool).await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("resetfamily")
            .description("Resets the family tree(s) in guild")
            .default_member_permissions(Permissions::ADMINISTRATOR)
    }
}
