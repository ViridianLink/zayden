use serenity::all::{ComponentInteraction, EditInteractionResponse, Http};
use sqlx::PgPool;

use super::Components;
use crate::{Result, actions};

impl Components {
    pub async fn join(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let (_, embed) = actions::join(http, interaction, pool, false).await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}
