use serenity::all::{ComponentInteraction, UserId};
use sqlx::{Database, Pool};
use zayden_core::message_metadata;

use crate::family_manager::FamilyManager;
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

    let mut row = Manager::row(pool, parent_user.id)
        .await?
        .unwrap_or_else(|| parent_user.into());

    let mut child_row = Manager::row(pool, child_user.id)
        .await?
        .unwrap_or_else(|| child_user.into());

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
