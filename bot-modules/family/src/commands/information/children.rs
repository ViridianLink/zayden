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
use zayden_core::as_u64;

use crate::family_manager::{FamilyManager, FamilyRow};
use crate::{FamilyError, Result};

pub struct Children;

impl Children {
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

        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        let row = Manager::row(pool, guild_id, user.id)
            .await?
            .unwrap_or_else(|| FamilyRow::from_user(guild_id, user));

        if row.children_ids.is_empty() {
            if user == &interaction.user {
                return Err(FamilyError::SelfNoChildren);
            }

            return Err(FamilyError::NoChildren(user.id));
        }

        let children: Vec<String> = stream::iter(row.children_ids)
            .then(|id| async move {
                let user_id = UserId::new(as_u64(id));
                let user = user_id.to_user(ctx).await?;

                Ok::<String, serenity::Error>(user.mention().to_string())
            })
            .try_collect()
            .await?;

        Ok((user.id, children))
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("children")
            .description("List who your children are.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user to check. Leave blank to check yourself.",
            ))
    }
}
