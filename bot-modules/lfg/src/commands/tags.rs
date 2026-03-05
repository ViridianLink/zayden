use std::collections::HashMap;

use serenity::all::{
    CommandInteraction, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    EditInteractionResponse, GuildChannel, GuildThread, Http, ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::{Error, PostManager, Result};

use super::Command;

impl Command {
    pub async fn tags<Db: Database, Manager: PostManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await.unwrap();

        let post_owner = Manager::owner(pool, interaction.channel_id).await.unwrap();

        if post_owner != interaction.user.id {
            return Err(Error::PermissionDenied(post_owner));
        }

        let thread_channel = interaction
            .channel_id
            .expect_thread()
            .to_thread(http, interaction.guild_id)
            .await
            .unwrap();

        let forum_channel = thread_channel
            .parent_id
            .to_guild_channel(http, interaction.guild_id)
            .await
            .unwrap();

        if options.contains_key("add") {
            add_tags(http, interaction, forum_channel, thread_channel)
                .await
                .unwrap();
        } else if options.contains_key("remove") {
            remove_tags(http, interaction, forum_channel, thread_channel)
                .await
                .unwrap();
        }

        Ok(())
    }
}
async fn add_tags(
    http: &Http,
    interaction: &CommandInteraction,
    forum_channel: GuildChannel,
    thread: GuildThread,
) -> Result<()> {
    let options = forum_channel
        .available_tags
        .into_iter()
        .filter(|tag| !thread.applied_tags.contains(&tag.id))
        .map(|tag| CreateSelectMenuOption::new(tag.name, tag.id.to_string()))
        .collect::<Vec<_>>();

    let max_values = options.len() as u8;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().select_menu(
                CreateSelectMenu::new(
                    "lfg_tags_add",
                    CreateSelectMenuKind::String {
                        options: options.into(),
                    },
                )
                .max_values(max_values),
            ),
        )
        .await
        .unwrap();

    Ok(())
}

async fn remove_tags(
    http: &Http,
    interaction: &CommandInteraction,
    forum_channel: GuildChannel,
    thread: GuildThread,
) -> Result<()> {
    let options = forum_channel
        .available_tags
        .into_iter()
        .filter(|tag| thread.applied_tags.contains(&tag.id))
        .map(|tag| CreateSelectMenuOption::new(tag.name, tag.id.to_string()))
        .collect::<Vec<_>>();

    let max_values = options.len() as u8;

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().select_menu(
                CreateSelectMenu::new(
                    "lfg_tags_remove",
                    CreateSelectMenuKind::String {
                        options: options.into(),
                    },
                )
                .max_values(max_values),
            ),
        )
        .await
        .unwrap();

    Ok(())
}
