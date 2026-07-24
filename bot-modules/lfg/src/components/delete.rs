use serenity::all::{ComponentInteraction, CreateInteractionResponse, Http};
use sqlx::PgPool;

use super::Components;
use crate::{LfgError, PostRow, Result, actions};

impl Components {
    pub async fn delete(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let owner = PostRow::fetch_owner(pool, interaction.channel_id).await?;

        if interaction.user.id != owner {
            return Err(LfgError::PermissionDenied(owner));
        }

        actions::delete(http, interaction.channel_id, pool).await?;

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await?;

        Ok(())
    }
}
