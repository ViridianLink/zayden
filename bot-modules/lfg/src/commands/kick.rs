use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    EditMessage,
    Http,
    Mentionable,
    ResolvedValue,
    User,
};
use sqlx::{Database, Pool};
use zayden_core::required_option;

use super::Command;
use crate::models::post::PostManager;
use crate::{LfgError, PostRow, Result, Savable, actions};

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
            Err(sqlx::Error::RowNotFound) => return Err(LfgError::ThreadNotFound),
            Err(e) => return Err(LfgError::Sqlx(e)),
        };
        if interaction.user.id != owner {
            return Err(LfgError::PermissionDenied(owner));
        }

        let (user, _): (&User, _) = required_option(&mut options, "user")?;

        let (thread, embed) =
            actions::leave::<Db, Manager>(http, interaction, pool, user.id).await?;

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
