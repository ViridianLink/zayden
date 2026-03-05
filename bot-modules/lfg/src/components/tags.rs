use std::collections::HashSet;

use serenity::all::{
    ComponentInteraction, ComponentInteractionDataKind, CreateInteractionResponse, EditThread,
    ForumTagId, Http,
};

use crate::Result;

pub struct TagsComponent;

impl TagsComponent {
    pub async fn add(http: &Http, interaction: &ComponentInteraction) -> Result<()> {
        let mut tag_ids = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values
                .iter()
                .map(|x| x.parse::<u64>().unwrap())
                .map(|id| ForumTagId::new(id))
                .collect::<Vec<_>>(),
            _ => unreachable!("Expected string select"),
        };

        let mut thread = interaction
            .channel_id
            .expect_thread()
            .to_thread(http, interaction.guild_id)
            .await
            .unwrap();

        tag_ids.extend(thread.applied_tags.iter().copied());

        thread
            .edit(http, EditThread::new().applied_tags(tag_ids))
            .await
            .unwrap();

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await
            .unwrap();

        Ok(())
    }

    pub async fn remove(http: &Http, interaction: &ComponentInteraction) -> Result<()> {
        let selected_ids = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values
                .iter()
                .map(|x| x.parse::<u64>().unwrap())
                .map(ForumTagId::new)
                .collect::<HashSet<_>>(),
            _ => unreachable!("Expected string select"),
        };

        let mut thread = interaction
            .channel_id
            .expect_thread()
            .to_thread(http, interaction.guild_id)
            .await
            .unwrap();

        let new_tag_ids = thread
            .applied_tags
            .iter()
            .copied()
            .filter(|tag_id| !selected_ids.contains(tag_id))
            .collect::<Vec<_>>();

        thread
            .edit(http, EditThread::new().applied_tags(new_tag_ids))
            .await
            .unwrap();

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await
            .unwrap();

        Ok(())
    }
}
