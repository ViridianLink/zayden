use serenity::all::{CommandInteraction, EditInteractionResponse, EditMessage, Http, Mentionable};
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

        let (thread, embed) = actions::leave::<Db, Manager>(http, interaction, pool)
            .await
            .unwrap();

        thread
            .widen()
            .edit_message(http, thread.get().into(), EditMessage::new().embed(embed))
            .await
            .unwrap();

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new()
                    .content(format!("You have left {}", thread.widen().mention())),
            )
            .await
            .unwrap();

        Ok(())
    }
}
