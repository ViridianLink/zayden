use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    Http,
    ResolvedValue,
    UserId,
};
use sqlx::PgPool;
use zayden_core::parse_options;

use crate::relationships::Relationships;
use crate::{FamilyError, FamilyRow, Result};

pub struct RelationshipResponse {
    pub other_id: UserId,
    pub user_id: UserId,
    pub relationship: Relationships,
}

pub struct Relationship;

impl Relationship {
    pub async fn run(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<RelationshipResponse> {
        interaction.defer(http).await?;

        let options = interaction.data.options();
        let options = parse_options(options);

        let Some(ResolvedValue::User(user, _)) = options.get("user") else {
            return Err(FamilyError::InvalidUserId);
        };
        let user = *user;

        let other = match options.get("other") {
            Some(ResolvedValue::User(user, _)) => *user,
            _ => &interaction.user,
        };

        if user == other {
            return Err(FamilyError::SameUser(user.id));
        }

        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        let user_info = FamilyRow::get(pool, guild_id, user.id)
            .await?
            .unwrap_or_else(|| FamilyRow::from_user(guild_id, user));

        let relationship = user_info.relationship(other.id);

        Ok(RelationshipResponse {
            other_id: other.id,
            user_id: user.id,
            relationship,
        })
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("relationship")
            .description("View the relationship between two users.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user",
                    "The user you want to view the relationship of.",
                )
                .required(true),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "other",
                "The other user you want to view the relationship of.",
            ))
    }
}
