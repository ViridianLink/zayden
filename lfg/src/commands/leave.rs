use serenity::all::{CommandInteraction, EditInteractionResponse, Http};
use sqlx::{Database, Pool};

use crate::{PostManager, PostRow, Result, Savable, actions};

use super::Command;

impl Command {
    pub async fn leave<Db: Database, Manager: PostManager<Db> + Savable<Db, PostRow>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await.unwrap();

        let content = actions::leave::<Db, Manager>(http, interaction, pool)
            .await
            .unwrap();

        interaction
            .edit_response(http, EditInteractionResponse::new().content(content))
            .await
            .unwrap();

        Ok(())
    }
}
