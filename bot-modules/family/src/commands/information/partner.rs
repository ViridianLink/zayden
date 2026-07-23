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
use sqlx::PgPool;
use zayden_core::as_u64;

use crate::{FamilyError, FamilyRow, Result};

pub struct Partner;

impl Partner {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<(UserId, Vec<String>)> {
        let user = match interaction.data.options().first() {
            Some(ResolvedOption { value: ResolvedValue::User(user, _), .. }) => {
                *user
            },
            _ => &interaction.user,
        };

        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        let row = FamilyRow::get(pool, guild_id, user.id)
            .await?
            .unwrap_or_else(|| FamilyRow::from_user(guild_id, user));

        if row.partner_ids.is_empty() {
            if user == &interaction.user {
                return Err(FamilyError::SelfNoPartners);
            }

            return Err(FamilyError::NoPartners(user.id));
        }

        let partners: Vec<String> = stream::iter(row.partner_ids)
            .then(|id| async move {
                let user_id = UserId::new(as_u64(id));
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
