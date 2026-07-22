use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::family_manager::{FamilyManager, FamilyRow};
use crate::{FamilyError, Result};

pub struct Block;

impl Block {
    pub async fn run<Db: Database, Manager: FamilyManager<Db>>(
        _ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let options = interaction.data.options();
        let option = options.first().ok_or(FamilyError::InvalidUserId)?;
        let ResolvedValue::User(user, _) = option.value else {
            return Err(FamilyError::InvalidUserId);
        };

        if &interaction.user == user {
            return Err(FamilyError::UserSelfBlock);
        }

        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        let mut row = Manager::row(pool, guild_id, interaction.user.id)
            .await?
            .unwrap_or_else(|| FamilyRow::from_user(guild_id, &interaction.user));

        row.add_blocked(user.id);
        row.save::<Db, Manager>(pool).await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("block")
            .description("Blocks a user from being able to adopt/marry/etc you.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user to block.",
            ))
    }
}

pub struct Unblock;

impl Unblock {
    pub async fn run<Db: Database, Manager: FamilyManager<Db>>(
        _ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let options = interaction.data.options();
        let option = options.first().ok_or(FamilyError::InvalidUserId)?;
        let ResolvedValue::User(user, _) = option.value else {
            return Err(FamilyError::InvalidUserId);
        };

        if &interaction.user == user {
            return Err(FamilyError::UserSelfBlock);
        }

        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        Manager::remove_block(pool, guild_id, interaction.user.id, user.id).await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("unblock")
            .description("Unblocks a user from being able to adopt/marry/etc you.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user to unblock.",
            ))
    }
}
