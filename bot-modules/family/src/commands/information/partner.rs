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

pub struct Partner;

impl Partner {
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

        if row.partner_ids.is_empty() {
            if user == &interaction.user {
                return Err(FamilyError::SelfNoPartners);
            }

            return Err(FamilyError::NoPartners(user.id));
        }

        let partners: Vec<String> = stream::iter(row.partner_ids)
            .then(|id| async move {
                let user_id = UserId::new(id.cast_unsigned());
                let user = user_id.to_user(ctx).await?;

                Ok::<String, serenity::Error>(user.mention().to_string())
            })
            .try_collect()
            .await?;

        Ok((user.id, partners))
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("partner")
            .description("List who you are married to.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user to check. Leave blank to check yourself.",
            ))
    }
}
