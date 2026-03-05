use serenity::all::{ComponentInteraction, MessageInteractionMetadata, UserId};
use sqlx::{Database, Pool};

use crate::family_manager::FamilyManager;
use crate::{Error, Result};

pub async fn accept<Db: Database, Manager: FamilyManager<Db>>(
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
) -> Result<UserId> {
    let parent_user = match interaction.message.interaction_metadata.as_deref() {
        Some(MessageInteractionMetadata::Command(metadata)) => &metadata.user,
        None => return Err(Error::NoInteraction),
        _ => unreachable!("Interaction metadata is not a CommandMetaData"),
    };

    let child_user = &interaction.user;

    if !interaction.message.mentions.contains(child_user) && child_user != parent_user {
        return Err(Error::UnauthorisedUser);
    };

    let mut row = match Manager::row(pool, parent_user.id).await? {
        Some(row) => row,
        None => parent_user.into(),
    };

    let mut child_row = match Manager::row(pool, child_user.id).await? {
        Some(row) => row,
        None => child_user.into(),
    };

    row.add_child(&child_row);
    child_row.add_parent(&row);

    row.save::<Db, Manager>(pool).await?;
    child_row.save::<Db, Manager>(pool).await?;

    Ok(parent_user.id)
}

pub async fn decline(interaction: &ComponentInteraction) -> Result<()> {
    if !interaction.message.mentions.contains(&interaction.user) {
        return Err(Error::UnauthorisedUser);
    }

    let command_author = match interaction.message.interaction_metadata.as_deref() {
        Some(MessageInteractionMetadata::Command(metadata)) => &metadata.user,
        None => return Err(Error::NoInteraction),
        _ => unreachable!("Interaction metadata is not a CommandMetaData"),
    };

    if command_author == &interaction.user {
        return Err(Error::AdoptCancelled);
    }

    Ok(())
}
