use serenity::all::{CommandInteraction, Context};
use sqlx::PgPool;
use tracing::warn;
use zayden_core::Autocomplete;

use crate::Result;
use crate::handler::Handler;
use crate::modules::destiny2::Perk;
use crate::modules::destiny2::endgame_analysis::slash_commands::{TierList, Weapon};
use crate::modules::lfg::Lfg;

impl Handler {
    pub async fn interaction_autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let option = interaction.data.autocomplete().unwrap();

        match interaction.data.name.as_str() {
            "lfg" => Lfg::autocomplete(ctx, interaction, option, pool).await,
            "perk" => Perk::autocomplete(ctx, interaction, option, pool).await,
            "weapon" => Weapon::autocomplete(ctx, interaction, option, pool).await,
            "tierlist" => TierList::autocomplete(ctx, interaction, option, pool).await,
            _ => {
                warn!("Unknown command: {}", interaction.data.name);
                Ok(())
            }
        }
    }
}
