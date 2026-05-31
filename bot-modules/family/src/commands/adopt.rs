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

pub struct Adopt;

impl Adopt {
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
            return Err(FamilyError::UserSelfAdopt);
        }

        if target_user.id == ctx.http.get_current_user().await?.id {
            return Err(FamilyError::Zayden);
        }

        if target_user.bot() {
            return Err(FamilyError::Bot);
        }

        let adopter_row = Manager::row(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| (&interaction.user).into());

        // Is already adopted?
        if let Some(target_row) = Manager::row(pool, target_user.id).await?
            && !target_row.parent_ids.is_empty()
        {
            return Err(FamilyError::AlreadyAdopted(target_user.id));
        }

        // Are the adopter and target are already related?
        let relationship = adopter_row.relationship(target_user.id);
        if relationship != Relationships::None {
            return Err(FamilyError::AlreadyRelated {
                target: target_user.id,
                relationship: Relationships::Parent,
            });
        }

        Ok(target_user.id)
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("adopt")
            .description("Adopt another user into your family.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user",
                    "The user to adopt.",
                )
                .required(true),
            )
    }
}
