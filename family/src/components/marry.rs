use serenity::all::{ComponentInteraction, MessageInteractionMetadata};
use sqlx::{Database, Pool};

use crate::family_manager::FamilyManager;
use crate::{Error, Result};

pub async fn accept<Db: Database, Manager: FamilyManager<Db>>(
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
) -> Result<()> {
    let author = match interaction.message.interaction_metadata.as_deref() {
        Some(MessageInteractionMetadata::Command(metadata)) => &metadata.user,
        None => return Err(Error::NoInteraction),
        _ => unreachable!("Interaction metadata is not a CommandMetaData"),
    };

    let partner = &interaction.user;

    if !interaction.message.mentions.contains(partner) && partner.id != author.id {
        return Err(Error::UnauthorisedUser);
    };

    let mut row = match Manager::row(pool, author.id).await? {
        Some(row) => row,
        None => author.into(),
    };

    let mut partner_row = match Manager::row(pool, partner.id).await? {
        Some(row) => row,
        None => partner.into(),
    };

    row.add_partner(&partner_row);
    partner_row.add_partner(&row);

    row.save::<Db, Manager>(pool).await?;
    partner_row.save::<Db, Manager>(pool).await?;

    Ok(())
}

pub async fn decline(interaction: &ComponentInteraction) -> Result<()> {
    if !interaction.message.mentions.contains(&interaction.user) {
        return Err(Error::UnauthorisedUser);
    }

    let author = match interaction.message.interaction_metadata.as_deref() {
        Some(MessageInteractionMetadata::Command(metadata)) => &metadata.user,
        None => return Err(Error::NoInteraction),
        _ => unreachable!("Interaction metadata is not a CommandMetaData"),
    };

    if author.id == interaction.user.id {
        return Err(Error::MarryCancelled);
    }

    Ok(())
}
