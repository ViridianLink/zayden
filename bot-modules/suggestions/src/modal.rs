use serenity::all::{
    AutoArchiveDuration,
    ChannelId,
    CreateEmbed,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateMessage,
    CreateThread,
    EditThread,
    Http,
    LabelComponent,
    MessageId,
    ModalComponent,
    ModalInteraction,
    ThreadId,
};

use crate::layout::{parse_link, thread_name};
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

        let (channel_id, message_id) =
            parse_link(old_url).ok_or(SuggestionsError::InvalidModalStructure)?;

        let prefix = if accepted { "[Accepted] - " } else { "[Rejected] - " };

        let name = if old_title.starts_with("[Accepted] - ")
            || old_title.starts_with("[Rejected] - ")
        {
            format!("{prefix}{}", old_title.get(11..).unwrap_or(old_title))
        } else {
            format!("{prefix}{old_title}").chars().take(100).collect::<String>()
        };

        let thread_id = suggestion_thread(http, channel_id, message_id).await?;

        thread_id.edit(http, EditThread::new().name(&name).archived(false)).await?;

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

        thread_id
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

async fn suggestion_thread(
    http: &Http,
    channel_id: ChannelId,
    message_id: MessageId,
) -> Result<ThreadId> {
    let thread_id = ThreadId::new(message_id.get());

    if channel_id.get() == message_id.get() {
        return Ok(thread_id);
    }

    let message = channel_id.widen().message(http, message_id).await?;

    if message.thread.is_some() {
        return Ok(thread_id);
    }

    let thread = channel_id
        .create_thread_from_message(
            http,
            message_id,
            CreateThread::new(thread_name(
                message.author.display_name(),
                &message.content,
            ))
            .auto_archive_duration(AutoArchiveDuration::OneWeek),
        )
        .await?;

    Ok(thread.id)
}
