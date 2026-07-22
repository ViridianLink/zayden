use serenity::all::{ComponentInteraction, UserId};
use sqlx::{Database, Pool};
use zayden_core::message_metadata;

use crate::family_manager::{FamilyManager, FamilyRow};
use crate::relationships::Relationships;
use crate::{FamilyError, Result};

pub async fn accept<Db: Database, Manager: FamilyManager<Db>>(
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
) -> Result<UserId> {
    let parent_user = &message_metadata(&interaction.message)?.user;

    let child_user = &interaction.user;

    if !interaction.message.mentions.contains(child_user)
        && child_user != parent_user
    {
        return Err(FamilyError::UnauthorisedUser);
    }

    let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let mut row = Manager::row(pool, guild_id, parent_user.id)
        .await?
        .unwrap_or_else(|| FamilyRow::from_user(guild_id, parent_user));

    let mut child_row = Manager::row(pool, guild_id, child_user.id)
        .await?
        .unwrap_or_else(|| FamilyRow::from_user(guild_id, child_user));

    if row.is_blocked(child_user.id) {
        return Err(FamilyError::Blocked(child_user.id));
    }
    if child_row.is_blocked(parent_user.id) {
        return Err(FamilyError::Blocked(parent_user.id));
    }

    if child_row.is_adopted() {
        return Err(FamilyError::AlreadyAdopted(child_user.id));
    }
    let relationship = row.relationship(child_user.id);
    if relationship != Relationships::None {
        return Err(FamilyError::AlreadyRelated {
            target: child_user.id,
            relationship,
        });
    }

    row.add_child(&child_row);
    child_row.add_parent(&row);

    row.save::<Db, Manager>(pool).await?;
    child_row.save::<Db, Manager>(pool).await?;

    Ok(parent_user.id)
}

pub fn decline(interaction: &ComponentInteraction) -> Result<()> {
    if !interaction.message.mentions.contains(&interaction.user) {
        return Err(FamilyError::UnauthorisedUser);
    }

    let command_author = &message_metadata(&interaction.message)?.user;

    if command_author == &interaction.user {
        return Err(FamilyError::AdoptCancelled);
    }

    Ok(())
}
