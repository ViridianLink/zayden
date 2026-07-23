use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    EditMessage,
    Http,
    Mentionable,
};
use sqlx::{Database, Pool};

use super::Command;
use crate::{PostManager, PostRow, Result, Savable, actions};

impl Command {
    pub async fn leave<
        Db: Database,
        Manager: PostManager<Db> + Savable<Db, PostRow>,
    >(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let (thread, embed) = actions::leave::<Db, Manager>(
            http,
            interaction,
            pool,
            interaction.user.id,
        )
        .await?;

        thread
            .widen()
            .edit_message(http, thread.get().into(), EditMessage::new().embed(embed))
            .await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new()
                    .content(format!("You have left {}", thread.widen().mention())),
            )
            .await?;

        Ok(())
    }
}
