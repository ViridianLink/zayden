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

use crate::family_manager::FamilyManager;
use crate::relationships::Relationships;
use crate::{FamilyError, Result};

const MAX_PARTNERS: usize = 1;

pub struct Marry;

impl Marry {
    pub async fn run<Db: Database, Manager: FamilyManager<Db>>(
        ctx: &Context,
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

        if target_user.id == ctx.http.get_current_user().await?.id {
            return Err(FamilyError::Zayden);
        }

        if target_user.bot() {
            return Err(FamilyError::Bot);
        }

        if let Some(row) = Manager::row(pool, interaction.user.id).await? {
            let relationship = row.relationship(target_user.id);

            if relationship != Relationships::None {
                return Err(FamilyError::AlreadyRelated {
                    target: target_user.id,
                    relationship,
                });
            }

            if row.partner_ids.len() >= MAX_PARTNERS {
                return Err(FamilyError::MaxPartners);
            }
        }

        if let Some(row) = Manager::row(pool, target_user.id).await?
            && row.partner_ids.len() >= MAX_PARTNERS
        {
            return Err(FamilyError::MaxPartners);
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
