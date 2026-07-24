use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    EditMessage,
    Http,
    Mentionable,
};
use sqlx::PgPool;

use super::Command;
use crate::{Result, actions};

impl Command {
    pub async fn leave(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let (thread, embed) =
            actions::leave(http, interaction, pool, interaction.user.id).await?;

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
