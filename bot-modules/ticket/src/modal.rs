use serenity::all::{
    AutoArchiveDuration,
    ChannelType,
    CreateEmbed,
    CreateEmbedFooter,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateMessage,
    CreateThread,
    Http,
    Mentionable,
    ModalInteraction,
};
use sqlx::PgPool;
use zayden_core::{CoreError, parse_modal_components};

use crate::ticket_manager::TicketRow;
use crate::{
    Result,
    TicketError,
    TicketGuildRow,
    TicketStores,
    send_support_message,
    thread_name,
    to_title_case,
};

pub struct TicketModal;

impl TicketModal {
    pub async fn run(
        http: &Http,
        interaction: &ModalInteraction,
        stores: TicketStores<'_>,
        pool: &PgPool,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(CoreError::MissingGuildId)?;

        let guild_row = TicketGuildRow::get(stores, pool, guild_id)
            .await?
            .ok_or(TicketError::SupportNotFound)?;

        let Some(message) = interaction.message.as_ref() else {
            return Err(TicketError::Internal(
                "TicketModal::run: interaction has no associated message".into(),
            ));
        };
        let ticket_row = TicketRow::get(pool, message.id).await?;
        let role_ids = ticket_row.role_ids();

        let mut data =
            parse_modal_components(interaction.data.components.as_slice());
        let content = data.remove("ticket_body").and_then(|mut v| v.pop()).ok_or(
            TicketError::ZaydenCore(CoreError::InvalidOption(
                "ticket_body".to_string(),
            )),
        )?;

        let member = interaction
            .member
            .as_ref()
            .ok_or(TicketError::ZaydenCore(CoreError::MissingGuildId))?;

        let thread_name =
            thread_name(guild_row.thread_id, member.display_name(), &content);

        let mut issue = CreateEmbed::new().title("Issue").description(content);

        if let Some(mut version) = data.remove("version")
            && let Some(v) = version.pop()
        {
            issue = issue.footer(CreateEmbedFooter::new(v));
        }

        let mut messages: Vec<CreateMessage<'_>> =
            vec![CreateMessage::new().embed(issue)];

        for (k, mut v) in data.drain().filter(|(_, v)| !v.is_empty()) {
            let title = to_title_case(&k);
            let description = v.pop().unwrap_or_default();
            let embed = CreateEmbed::new().title(title).description(description);
            messages.push(CreateMessage::new().embed(embed));
        }

        let mut additional = data.remove("additional").unwrap_or_default();
        if !additional.is_empty() {
            let description = additional.pop().unwrap_or_default();
            let additional = CreateEmbed::new()
                .title("Additional Information")
                .description(description);

            if let Some(msg) = messages.get_mut(1) {
                *msg = CreateMessage::new().embed(additional);
            }
        }

        let thread = interaction
            .channel_id
            .expect_channel()
            .create_thread(
                http,
                CreateThread::new(&thread_name)
                    .kind(ChannelType::PrivateThread)
                    .auto_archive_duration(AutoArchiveDuration::OneWeek),
            )
            .await?;

        TicketGuildRow::increment_thread_id(stores.ticket, guild_id).await?;

        let mentions = if role_ids.is_empty() {
            let owner_id = guild_id.to_partial_guild(http).await?.owner_id;
            vec![interaction.user.mention(), owner_id.mention()]
        } else {
            role_ids
                .into_iter()
                .map(|id| id.mention())
                .chain([interaction.user.mention()])
                .collect::<Vec<_>>()
        };

        send_support_message(http, thread.id, &mentions, messages).await?;

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "Support thread created: {}",
                            thread.mention()
                        ))
                        .ephemeral(true),
                ),
            )
            .await?;

        Ok(())
    }
}
