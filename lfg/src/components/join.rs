use serenity::all::{ComponentInteraction,  CreateInteractionResponse, Http};
use sqlx::{Database, Pool};

use crate::{PostManager, PostRow, Result, Savable, actions};

use super::Components;

impl Components {
    pub async fn join<Db: Database, Manager: PostManager<Db> + Savable<Db, PostRow>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        actions::join::<Db, Manager>(http, interaction, pool, false).await?;

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await
            .unwrap();

        Ok(())
    }
}
