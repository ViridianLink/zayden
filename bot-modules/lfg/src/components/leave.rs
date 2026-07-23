use serenity::all::{ComponentInteraction, EditInteractionResponse, Http};
use sqlx::{Database, Pool};

use super::Components;
use crate::{PostManager, PostRow, Result, Savable, actions};

impl Components {
    pub async fn leave<
        Db: Database,
        Manager: PostManager<Db> + Savable<Db, PostRow>,
    >(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let (_, embed) = actions::leave::<Db, Manager>(
            http,
            interaction,
            pool,
            interaction.user.id,
        )
        .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}
