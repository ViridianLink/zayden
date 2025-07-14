use serenity::all::{ComponentInteraction,  CreateInteractionResponse, Http};
use sqlx::{Database, Pool};

use crate::{PostManager, PostRow, Result, Savable, actions};

use super::Components;

impl Components {
    pub async fn leave<Db: Database, Manager: PostManager<Db> + Savable<Db, PostRow>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        actions::leave::<Db, Manager>(http, interaction, pool)
            .await
            .unwrap();

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await
            .unwrap();

        Ok(())
    }
}
