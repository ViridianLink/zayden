use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    ResolvedValue,
    UserId,
};
use sqlx::PgPool;
use zayden_core::as_i64;

use crate::{FamilyError, FamilyRow, Result};

pub struct Divorce;

impl Divorce {
    pub async fn run(
        _ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<UserId> {
        let options = interaction.data.options();
        let option = options.first().ok_or(FamilyError::InvalidUserId)?;
        let ResolvedValue::User(target_user, _) = option.value else {
            return Err(FamilyError::InvalidUserId);
        };

        if interaction.user.id == target_user.id {
            return Err(FamilyError::UserSelfMarry);
        }

        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        let row = FamilyRow::get(pool, guild_id, interaction.user.id)
            .await?
            .ok_or(FamilyError::SelfNoPartners)?;

        if !row.partner_ids.contains(&as_i64(target_user.id.get())) {
            return Err(FamilyError::NotPartners(target_user.id));
        }

        FamilyRow::remove_partner(
            pool,
            guild_id,
            interaction.user.id,
            target_user.id,
        )
        .await?;

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
