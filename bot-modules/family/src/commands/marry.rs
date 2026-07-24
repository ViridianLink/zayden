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

use crate::relationships::Relationships;
use crate::{FamilyError, FamilyRow, FamilySettings, Result};

pub struct Marry;

impl Marry {
    pub async fn run(
        ctx: &Context,
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

        if target_user.id == ctx.http.get_current_user().await?.id {
            return Err(FamilyError::Zayden);
        }

        if target_user.bot() {
            return Err(FamilyError::Bot);
        }

        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        let max_partners = FamilySettings::get(pool, guild_id).await?.max_partners();

        if let Some(row) =
            FamilyRow::get(pool, guild_id, interaction.user.id).await?
        {
            let relationship = row.relationship(target_user.id);

            if relationship != Relationships::None {
                return Err(FamilyError::AlreadyRelated {
                    target: target_user.id,
                    relationship,
                });
            }

            if row.at_partner_limit(max_partners) {
                return Err(FamilyError::MaxPartners);
            }

            if row.is_blocked(target_user.id) {
                return Err(FamilyError::Blocked(target_user.id));
            }
        }

        if let Some(row) = FamilyRow::get(pool, guild_id, target_user.id).await? {
            if row.at_partner_limit(max_partners) {
                return Err(FamilyError::MaxPartners);
            }

            if row.is_blocked(interaction.user.id) {
                return Err(FamilyError::Blocked(interaction.user.id));
            }
        }

        Ok(target_user.id)
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("marry")
            .description("Propose to another Discord user")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user",
                    "The user you want to propose to",
                )
                .required(true),
            )
    }
}
