use std::collections::HashSet;

use serenity::all::{
    ComponentInteraction,
    ComponentInteractionDataKind,
    CreateInteractionResponse,
    EditThread,
    ForumTagId,
    Http,
};
use tracing::error;

use crate::Result;

pub struct TagsComponent;

impl TagsComponent {
    pub async fn add(http: &Http, interaction: &ComponentInteraction) -> Result<()> {
        let mut tag_ids = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values
                .iter()
                .map(|x| {
                    x.parse::<u64>().expect("forum tag ID is always a valid u64")
                })
                .map(ForumTagId::new)
                .collect::<Vec<_>>(),
            ComponentInteractionDataKind::Button
            | ComponentInteractionDataKind::UserSelect { .. }
            | ComponentInteractionDataKind::RoleSelect { .. }
            | ComponentInteractionDataKind::MentionableSelect { .. }
            | ComponentInteractionDataKind::ChannelSelect { .. }
            | ComponentInteractionDataKind::Unknown(_) => {
                error!("Expected string select");
                return Ok(());
            },
        };

        let mut thread = interaction
            .channel_id
            .expect_thread()
            .to_thread(http, interaction.guild_id)
            .await?;

        tag_ids.extend(thread.applied_tags.iter().copied());

        thread.edit(http, EditThread::new().applied_tags(tag_ids)).await?;

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await?;

        Ok(())
    }

    pub async fn remove(
        http: &Http,
        interaction: &ComponentInteraction,
    ) -> Result<()> {
        let selected_ids = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values
                .iter()
                .map(|x| {
                    x.parse::<u64>().expect("forum tag ID is always a valid u64")
                })
                .map(ForumTagId::new)
                .collect::<HashSet<_>>(),
            ComponentInteractionDataKind::Button
            | ComponentInteractionDataKind::UserSelect { .. }
            | ComponentInteractionDataKind::RoleSelect { .. }
            | ComponentInteractionDataKind::MentionableSelect { .. }
            | ComponentInteractionDataKind::ChannelSelect { .. }
            | ComponentInteractionDataKind::Unknown(_) => {
                error!("Expected string select");
                return Ok(());
            },
        };

        let mut thread = interaction
            .channel_id
            .expect_thread()
            .to_thread(http, interaction.guild_id)
            .await?;

        let new_tag_ids = thread
            .applied_tags
            .iter()
            .copied()
            .filter(|tag_id| !selected_ids.contains(tag_id))
            .collect::<Vec<_>>();

        thread.edit(http, EditThread::new().applied_tags(new_tag_ids)).await?;

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await?;

        Ok(())
    }
}
