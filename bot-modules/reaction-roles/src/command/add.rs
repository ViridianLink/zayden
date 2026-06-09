use std::collections::HashMap;

use serenity::all::{
    CreateEmbed,
    CreateMessage,
    GenericChannelId,
    GuildId,
    Http,
    Mentionable,
    MessageId,
    ReactionType,
    ResolvedValue,
    Role,
};
use sqlx::{Database, Pool};
use zayden_core::required_option;

use super::ReactionRoleCommand;
use crate::reaction_roles_manager::ReactionRolesManager;
use crate::{ReactionRoleError, Result};

impl ReactionRoleCommand {
    pub(super) async fn add<Db: Database, Manager: ReactionRolesManager<Db>>(
        http: &Http,
        pool: &Pool<Db>,
        guild_id: GuildId,
        channel_id: GenericChannelId,
        reaction: ReactionType,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        let role: &Role = required_option(&mut options, "role")?;

        let message_id = match options.remove("message_id") {
            Some(ResolvedValue::String(id)) => {
                Some(MessageId::new(id.parse().map_err(|_e| {
                    ReactionRoleError::InvalidMessageId(id.to_string())
                })?))
            },
            _ => None,
        };

        let message = match message_id {
            Some(message_id) => channel_id.message(http, message_id).await?,
            None => {
                channel_id
                    .send_message(
                        http,
                        CreateMessage::new().embed(CreateEmbed::new().description(
                            format!("{} | {}", reaction, role.mention()),
                        )),
                    )
                    .await?
            },
        };

        Manager::create(
            pool,
            guild_id,
            channel_id,
            message.id,
            role.id,
            &reaction.to_string(),
        )
        .await?;

        message.react(http, reaction).await?;

        Ok(())
    }
}
