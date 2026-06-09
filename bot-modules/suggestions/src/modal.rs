use serenity::all::{
    CreateEmbed,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateMessage,
    EditThread,
    Http,
    LabelComponent,
    ModalComponent,
    ModalInteraction,
    ThreadId,
};

use crate::{Result, Suggestions, SuggestionsError};

impl Suggestions {
    pub async fn modal(
        http: &Http,
        modal: &ModalInteraction,
        accepted: bool,
    ) -> Result<()> {
        let response = match modal.data.components.first() {
            Some(ModalComponent::Label(label)) => match &label.component {
                LabelComponent::InputText(input_text) => &input_text.value,
                LabelComponent::SelectMenu(_)
                | LabelComponent::FileUpload(_)
                | LabelComponent::RadioGroup(_)
                | LabelComponent::CheckboxGroup(_)
                | LabelComponent::Checkbox(_)
                | _ => return Err(SuggestionsError::InvalidModalStructure),
            },
            _ => return Err(SuggestionsError::InvalidModalStructure),
        };

        let old_embed = modal
            .message
            .as_ref()
            .ok_or(SuggestionsError::InvalidModalStructure)?
            .embeds
            .first()
            .ok_or(SuggestionsError::InvalidModalStructure)?;

        let old_url = old_embed
            .url
            .as_deref()
            .ok_or(SuggestionsError::InvalidModalStructure)?;
        let old_title = old_embed
            .title
            .as_deref()
            .ok_or(SuggestionsError::InvalidModalStructure)?;

        let channel_id = old_url
            .split('/')
            .nth(5)
            .ok_or(SuggestionsError::InvalidModalStructure)?
            .parse::<ThreadId>()
            .map_err(|_e| SuggestionsError::InvalidModalStructure)?;

        let prefix = if accepted { "[Accepted] - " } else { "[Rejected] - " };

        let name = if old_title.starts_with("[Accepted] - ")
            || old_title.starts_with("[Rejected] - ")
        {
            format!("{prefix}{}", old_title.get(11..).unwrap_or(old_title))
        } else {
            format!("{prefix}{old_title}").chars().take(100).collect::<String>()
        };

        channel_id.edit(http, EditThread::new().name(&name).archived(false)).await?;

        let old_description = old_embed
            .description
            .as_deref()
            .ok_or(SuggestionsError::InvalidModalStructure)?;
        let old_author = old_embed
            .author
            .as_ref()
            .ok_or(SuggestionsError::InvalidModalStructure)?
            .clone();
        let old_footer = old_embed
            .footer
            .as_ref()
            .ok_or(SuggestionsError::InvalidModalStructure)?
            .clone();

        modal
            .create_response(
                http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .title(name)
                            .url(old_url)
                            .description(old_description)
                            .field("Team Response", response, false)
                            .author(old_author.into())
                            .footer(old_footer.into()),
                    ),
                ),
            )
            .await?;

        let title =
            if accepted { "Suggestion Accepted" } else { "Suggestion Rejected" };

        channel_id
            .widen()
            .send_message(
                http,
                CreateMessage::new()
                    .embed(CreateEmbed::new().title(title).description(response)),
            )
            .await?
            .pin(http, Some("Mod response pinned"))
            .await?;

        Ok(())
    }
}
