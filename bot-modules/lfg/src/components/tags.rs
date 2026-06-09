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

use crate::{LfgError, Result};

pub struct TagsComponent;

impl TagsComponent {
    pub async fn add(http: &Http, interaction: &ComponentInteraction) -> Result<()> {
        let mut tag_ids = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values
                .iter()
                .filter_map(|x| {
                    x.parse::<u64>()
                        .map_err(|e| {
                            error!("Failed to parse forum tag ID '{}': {e}", x);
                        })
                        .ok()
                        .map(ForumTagId::new)
                })
                .collect::<Vec<_>>(),
            ComponentInteractionDataKind::Button
            | ComponentInteractionDataKind::UserSelect { .. }
            | ComponentInteractionDataKind::RoleSelect { .. }
            | ComponentInteractionDataKind::MentionableSelect { .. }
            | ComponentInteractionDataKind::ChannelSelect { .. }
            | ComponentInteractionDataKind::Unknown(_) => {
                return Err(LfgError::Internal(
                    "TagsComponent::add: expected StringSelect interaction".into(),
                ));
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
                .filter_map(|x| {
                    x.parse::<u64>()
                        .map_err(|e| {
                            error!("Failed to parse forum tag ID '{}': {e}", x);
                        })
                        .ok()
                        .map(ForumTagId::new)
                })
                .collect::<HashSet<_>>(),
            ComponentInteractionDataKind::Button
            | ComponentInteractionDataKind::UserSelect { .. }
            | ComponentInteractionDataKind::RoleSelect { .. }
            | ComponentInteractionDataKind::MentionableSelect { .. }
            | ComponentInteractionDataKind::ChannelSelect { .. }
            | ComponentInteractionDataKind::Unknown(_) => {
                return Err(LfgError::Internal(
                    "TagsComponent::remove: expected StringSelect interaction"
                        .into(),
                ));
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
