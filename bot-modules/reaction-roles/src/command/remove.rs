use std::collections::HashMap;

use serenity::all::{
    GenericChannelId,
    GuildId,
    Http,
    MessageId,
    ReactionType,
    ResolvedValue,
};
use sqlx::PgPool;
use zayden_core::required_option;

use super::ReactionRoleCommand;
use crate::{ReactionRole, ReactionRoleError, Result};

impl ReactionRoleCommand {
    pub(super) async fn remove(
        http: &Http,
        pool: &PgPool,
        channel_id: GenericChannelId,
        guild_id: GuildId,
        reaction: ReactionType,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        let id: &str = required_option(&mut options, "message_id")?;
        let message_id =
            MessageId::new(id.parse().map_err(|_e| {
                ReactionRoleError::InvalidMessageId(id.to_string())
            })?);

        ReactionRole::delete(
            pool,
            guild_id,
            channel_id,
            message_id,
            &reaction.to_string(),
        )
        .await?;

        channel_id.delete_reaction_emoji(http, message_id, reaction).await?;

        Ok(())
    }
}
