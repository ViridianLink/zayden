use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    EditMessage,
    Http,
    Mentionable,
    ResolvedValue,
};
use sqlx::PgPool;

use super::Command;
use crate::{Result, actions};

impl Command {
    pub async fn join(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &PgPool,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let alternative = match options.remove("alternative") {
            Some(ResolvedValue::Boolean(alt)) => alt,
            _ => false,
        };

        let (thread, embed) =
            actions::join(http, interaction, pool, alternative).await?;

        thread
            .widen()
            .edit_message(http, thread.get().into(), EditMessage::new().embed(embed))
            .await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content(format!(
                    "You have joined {}",
                    thread.widen().mention()
                )),
            )
            .await?;

        Ok(())
    }
}
