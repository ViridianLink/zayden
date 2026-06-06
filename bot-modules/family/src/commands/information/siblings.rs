use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    Mentionable,
    ResolvedOption,
    ResolvedValue,
    UserId,
};
use sqlx::{Database, Pool};
use zayden_core::{as_i64, as_u64};

use crate::family_manager::FamilyManager;
use crate::{FamilyError, Result};

pub struct Siblings;

impl Siblings {
    pub async fn run<Db: Database, Manager: FamilyManager<Db>>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<(UserId, Vec<String>)> {
        let user = match interaction.data.options().first() {
            Some(ResolvedOption { value: ResolvedValue::User(user, _), .. }) => {
                *user
            },
            _ => &interaction.user,
        };

        let row = Manager::row(pool, user.id)
            .await?
            .unwrap_or_else(|| (&interaction.user).into());

        if row.parent_ids.is_empty() {
            if user == &interaction.user {
                return Err(FamilyError::SelfNoSiblings);
            }
            return Err(FamilyError::NoSiblings(user.id));
        }

        let user_id_signed: i64 = as_i64(user.id.get());
        let mut siblings = Vec::new();

        for parent_id in row.parent_ids {
            let parent_uid = UserId::new(as_u64(parent_id));
            if let Some(parent_row) = Manager::row(pool, parent_uid).await? {
                for sib_id in parent_row.children_ids {
                    if sib_id != user_id_signed {
                        let sib_uid = UserId::new(as_u64(sib_id));
                        let sib_user = sib_uid.to_user(ctx).await?;
                        siblings.push(sib_user.mention().to_string());
                    }
                }
            }
        }

        if siblings.is_empty() {
            if user == &interaction.user {
                return Err(FamilyError::SelfNoSiblings);
            }
            return Err(FamilyError::NoSiblings(user.id));
        }

        Ok((user.id, siblings))
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("siblings")
            .description("List who your siblings are.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user to check. Leave blank to check yourself.",
            ))
    }
}
