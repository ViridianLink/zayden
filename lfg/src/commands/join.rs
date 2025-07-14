use std::collections::HashMap;

use serenity::all::{CommandInteraction, EditInteractionResponse, Http, ResolvedValue};
use sqlx::{Database, Pool};

use crate::{PostManager, PostRow, Result, Savable, actions};

use super::Command;

impl Command {
    pub async fn join<Db: Database, Manager: PostManager<Db> + Savable<Db, PostRow>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await.unwrap();

        let alternative = match options.remove("alternative") {
            Some(ResolvedValue::Boolean(alt)) => alt,
            _ => false,
        };

        let content = actions::join::<Db, Manager>(http, interaction, pool, alternative).await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(content))
            .await
            .unwrap();

        Ok(())
    }
}
