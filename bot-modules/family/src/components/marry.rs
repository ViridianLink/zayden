use serenity::all::ComponentInteraction;
use sqlx::{Database, Pool};
use zayden_core::message_metadata;

use crate::family_manager::{FamilyManager, FamilyRow};
use crate::relationships::Relationships;
use crate::{FamilyError, Result};

pub async fn accept<Db: Database, Manager: FamilyManager<Db>>(
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
) -> Result<()> {
    let author = &message_metadata(&interaction.message)?.user;

    let partner = &interaction.user;

    if !interaction.message.mentions.contains(partner) && partner.id != author.id {
        return Err(FamilyError::UnauthorisedUser);
    }

    let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let max_partners = Manager::settings(pool, guild_id).await?.max_partners();

    let mut row = Manager::row(pool, guild_id, author.id)
        .await?
        .unwrap_or_else(|| FamilyRow::from_user(guild_id, author));

    let mut partner_row = Manager::row(pool, guild_id, partner.id)
        .await?
        .unwrap_or_else(|| FamilyRow::from_user(guild_id, partner));

    if row.is_blocked(partner.id) {
        return Err(FamilyError::Blocked(partner.id));
    }
    if partner_row.is_blocked(author.id) {
        return Err(FamilyError::Blocked(author.id));
    }

    let relationship = row.relationship(partner.id);
    if relationship != Relationships::None {
        return Err(FamilyError::AlreadyRelated {
            target: partner.id,
            relationship,
        });
    }
    if row.at_partner_limit(max_partners)
        || partner_row.at_partner_limit(max_partners)
    {
        return Err(FamilyError::MaxPartners);
    }

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

    let author = &message_metadata(&interaction.message)?.user;

    if author.id == interaction.user.id {
        return Err(FamilyError::MarryCancelled);
    }

    Ok(())
}
