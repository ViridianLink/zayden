use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    ResolvedValue,
    UserId,
};
use sqlx::{Database, Pool};
use zayden_core::as_i64;

use crate::family_manager::FamilyManager;
use crate::{FamilyError, Result};

pub struct Divorce;

impl Divorce {
    pub async fn run<Db: Database, Manager: FamilyManager<Db>>(
        _ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<UserId> {
        let options = interaction.data.options();
        let option = options.first().ok_or(FamilyError::InvalidUserId)?;
        let ResolvedValue::User(target_user, _) = option.value else {
            return Err(FamilyError::InvalidUserId);
        };

        if interaction.user.id == target_user.id {
            return Err(FamilyError::UserSelfMarry);
        }

        let row = Manager::row(pool, interaction.user.id)
            .await?
            .ok_or(FamilyError::SelfNoPartners)?;

        if !row.partner_ids.contains(&as_i64(target_user.id.get())) {
            return Err(FamilyError::NotPartners(target_user.id));
        }

        Manager::remove_partner(pool, interaction.user.id, target_user.id).await?;

        Ok(target_user.id)
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("divorce").description("Divorce your partner").add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The partner to divorce",
            )
            .required(true),
        )
    }
}
