use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    EditMessage,
    Http,
    Mentionable,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use super::Command;
use crate::models::post::PostManager;
use crate::{Error, PostRow, Result, Savable, actions};

impl Command {
    pub async fn kick<
        Db: Database,
        Manager: PostManager<Db> + Savable<Db, PostRow>,
    >(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let thread = match options.remove("thread") {
            Some(ResolvedValue::Channel(channel)) => channel.id(),
            _ => interaction.channel_id,
        };

        let owner = match Manager::owner(pool, thread).await {
            Ok(owner) => owner,
            Err(sqlx::Error::RowNotFound) => return Err(Error::ThreadNotFound),
            Err(e) => return Err(Error::Sqlx(e)),
        };
        if interaction.user.id != owner {
            return Err(Error::PermissionDenied(owner));
        }

        #[expect(
            clippy::unreachable,
            reason = "Discord guarantees required options are present"
        )]
        let Some(ResolvedValue::User(user, _)) = options.remove("user") else {
            unreachable!("User option is required")
        };

        let (thread, embed) =
            actions::leave::<Db, Manager>(http, interaction, pool, user).await?;

        thread
            .widen()
            .edit_message(http, thread.get().into(), EditMessage::new().embed(embed))
            .await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content(format!(
                    "You have kicked {} ({})",
                    user.mention(),
                    user.display_name()
                )),
            )
            .await?;

        Ok(())
    }
}
