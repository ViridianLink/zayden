use serenity::all::{ComponentInteraction, MessageInteractionMetadata};
use sqlx::{Database, Pool};

use crate::family_manager::FamilyManager;
use crate::{FamilyError, Result};

pub async fn accept<Db: Database, Manager: FamilyManager<Db>>(
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
) -> Result<()> {
    let author = match interaction.message.interaction_metadata.as_deref() {
        Some(MessageInteractionMetadata::Command(metadata)) => &metadata.user,
        None => return Err(FamilyError::NoInteraction),
        Some(_) => return Err(FamilyError::InvalidUserId),
    };

    let partner = &interaction.user;

    if !interaction.message.mentions.contains(partner) && partner.id != author.id {
        return Err(FamilyError::UnauthorisedUser);
    }

    let mut row =
        Manager::row(pool, author.id).await?.unwrap_or_else(|| author.into());

    let mut partner_row =
        Manager::row(pool, partner.id).await?.unwrap_or_else(|| partner.into());

    row.add_partner(&partner_row);
    partner_row.add_partner(&row);

    row.save::<Db, Manager>(pool).await?;
    partner_row.save::<Db, Manager>(pool).await?;

    Ok(())
}

pub fn decline(interaction: &ComponentInteraction) -> Result<()> {
    if !interaction.message.mentions.contains(&interaction.user) {
        return Err(FamilyError::UnauthorisedUser);
    }

    let author = match interaction.message.interaction_metadata.as_deref() {
        Some(MessageInteractionMetadata::Command(metadata)) => &metadata.user,
        None => return Err(FamilyError::NoInteraction),
        Some(_) => return Err(FamilyError::InvalidUserId),
    };

    if author.id == interaction.user.id {
        return Err(FamilyError::MarryCancelled);
    }

    Ok(())
}
