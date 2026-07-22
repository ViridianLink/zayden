use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    CreateSelectMenu,
    CreateSelectMenuKind,
    CreateSelectMenuOption,
    EditInteractionResponse,
    ForumTagId,
    GuildChannel,
    GuildThread,
    Http,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use super::Command;
use crate::{LfgError, PostManager, Result};

impl Command {
    pub async fn tags<Db: Database, Manager: PostManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let post_owner = Manager::owner(pool, interaction.channel_id).await?;

        if post_owner != interaction.user.id {
            return Err(LfgError::PermissionDenied(post_owner));
        }

        let thread_channel = interaction
            .channel_id
            .expect_thread()
            .to_thread(http, interaction.guild_id)
            .await?;

        let forum_channel = thread_channel
            .parent_id
            .to_guild_channel(http, interaction.guild_id)
            .await?;

        if options.contains_key("add") {
            edit_tags(
                http,
                interaction,
                forum_channel,
                thread_channel,
                TagAction::Add,
            )
            .await?;
        } else if options.contains_key("remove") {
            edit_tags(
                http,
                interaction,
                forum_channel,
                thread_channel,
                TagAction::Remove,
            )
            .await?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
pub enum TagAction {
    Add,
    Remove,
}

impl TagAction {
    const fn custom_id(self) -> &'static str {
        match self {
            Self::Add => "lfg_tags_add",
            Self::Remove => "lfg_tags_remove",
        }
    }

    const fn includes(self, applied: bool) -> bool {
        match self {
            Self::Add => !applied,
            Self::Remove => applied,
        }
    }

    const fn empty_notice(self) -> &'static str {
        match self {
            Self::Add => "This post already has every available tag.",
            Self::Remove => "This post has no tags to remove.",
        }
    }
}

pub enum TagResponse {
    Notice(&'static str),
    Menu(Vec<CreateSelectMenuOption<'static>>),
}

pub fn build_tag_response(
    available_tags: impl IntoIterator<Item = (ForumTagId, String)>,
    applied_tags: &[ForumTagId],
    action: TagAction,
) -> TagResponse {
    let options = available_tags
        .into_iter()
        .filter(|(id, _)| action.includes(applied_tags.contains(id)))
        .map(|(id, name)| CreateSelectMenuOption::new(name, id.to_string()))
        .collect::<Vec<_>>();

    if options.is_empty() {
        TagResponse::Notice(action.empty_notice())
    } else {
        TagResponse::Menu(options)
    }
}

async fn edit_tags(
    http: &Http,
    interaction: &CommandInteraction,
    forum_channel: GuildChannel,
    thread: GuildThread,
    action: TagAction,
) -> Result<()> {
    let available = forum_channel
        .available_tags
        .into_iter()
        .map(|tag| (tag.id, tag.name.to_string()));

    let response = match build_tag_response(available, &thread.applied_tags, action)
    {
        TagResponse::Notice(notice) => {
            EditInteractionResponse::new().content(notice)
        },
        TagResponse::Menu(options) => {
            let max_values = u8::try_from(options.len()).unwrap_or(u8::MAX);

            EditInteractionResponse::new().select_menu(
                CreateSelectMenu::new(
                    action.custom_id(),
                    CreateSelectMenuKind::String { options: options.into() },
                )
                .max_values(max_values),
            )
        },
    };

    interaction.edit_response(http, response).await?;

    Ok(())
}
