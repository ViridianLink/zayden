use futures::{StreamExt, TryStreamExt, stream};
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

use crate::family_manager::FamilyManager;
use crate::{FamilyError, Result};

pub struct Siblings;

impl Siblings {
    #[expect(clippy::cast_sign_loss, reason = "stored IDs are always non-negative")]
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
                return Err(FamilyError::SelfNoParents);
            }

            return Err(FamilyError::NoParents(user.id));
        }

        let siblings: Vec<String> = stream::iter(row.parent_ids)
            .map(|id| UserId::new(id.cast_unsigned()))
            .then(|id| async move {
                if let Some(row) = Manager::row(pool, id).await? {
                    for sib_id in row.children_ids {
                        if sib_id != row.id {
                            let user_id = UserId::new(sib_id.cast_unsigned());
                            let user = user_id.to_user(ctx).await?;

                            return Ok::<String, FamilyError>(
                                user.mention().to_string(),
                            );
                        }
                    }
                }

                Err(FamilyError::NoData(id))
            })
            .try_collect()
            .await?;

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
