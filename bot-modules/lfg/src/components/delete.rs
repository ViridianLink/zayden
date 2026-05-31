use serenity::all::{ComponentInteraction, CreateInteractionResponse, Http};
use sqlx::{Database, Pool};

use super::Components;
use crate::{LfgError, PostManager, Result, actions};

impl Components {
    pub async fn delete<Db: Database, Manager: PostManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let owner = Manager::owner(pool, interaction.channel_id).await?;

        if interaction.user.id != owner {
            return Err(LfgError::PermissionDenied(owner));
        }

        actions::delete::<Db, Manager>(http, interaction.channel_id, pool).await?;

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await?;

        Ok(())
    }
}
