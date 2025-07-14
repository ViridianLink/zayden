use serenity::all::{ComponentInteraction,  CreateInteractionResponse, Http};
use sqlx::{Database, Pool};

use crate::{Error, PostManager, Result, actions};

use super::Components;

impl Components {
    pub async fn delete<Db: Database, Manager: PostManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let owner = Manager::owner(pool, interaction.channel_id).await?;

        if interaction.user.id != owner {
            return Err(Error::PermissionDenied(owner));
        }

        actions::delete::<Db, Manager>(http, interaction.channel_id, pool)
            .await
            .unwrap();

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await
            .unwrap();

        Ok(())
    }
}
