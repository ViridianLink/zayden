use serenity::all::{
    ActionRowComponent, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, EditThread, Http, ModalInteraction, ThreadId,
};

use crate::Suggestions;

impl Suggestions {
    pub async fn modal(http: &Http, modal: &ModalInteraction, accepted: bool) {
        let ActionRowComponent::InputText(text) = &modal
            .data
            .components
            .first()
            .unwrap()
            .components
            .first()
            .unwrap()
        else {
            unreachable!("InputText is required");
        };

        let response = text.value.as_deref().unwrap();

        let old_embed = modal.message.as_ref().unwrap().embeds.first().unwrap();
        let old_url = old_embed.url.as_deref().unwrap();
        let old_title = old_embed.title.as_deref().unwrap();

        let channel_id = old_url
            .split('/')
            .nth(5)
            .unwrap()
            .parse::<ThreadId>()
            .unwrap();

        let prefix = if accepted {
            "[Accepted] - "
        } else {
            "[Rejected] - "
        };

        let name =
            if old_title.starts_with("[Accepted] - ") || old_title.starts_with("[Rejected] - ") {
                format!("{prefix}{}", &old_title[11..])
            } else {
                format!("{prefix}{old_title}")
                    .chars()
                    .take(100)
                    .collect::<String>()
            };

        channel_id
            .edit(http, EditThread::new().name(&name).archived(false))
            .await
            .unwrap();

        modal
            .create_response(
                http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .title(name)
                            .url(old_url)
                            .description(old_embed.description.as_deref().unwrap())
                            .field("Team Response", response, false)
                            .author(old_embed.author.as_ref().unwrap().clone().into())
                            .footer(old_embed.footer.as_ref().unwrap().clone().into()),
                    ),
                ),
            )
            .await
            .unwrap();

        let title = if accepted {
            "Suggestion Accepted"
        } else {
            "Suggestion Rejected"
        };

        channel_id
            .widen()
            .send_message(
                http,
                CreateMessage::new().embed(CreateEmbed::new().title(title).description(response)),
            )
            .await
            .unwrap()
            .pin(http, Some("Mod response pinned"))
            .await
            .unwrap();
    }
}
