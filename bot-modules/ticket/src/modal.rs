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
use sqlx::{Database, Pool};
use zayden_core::parse_modal_components;

use crate::ticket_manager::TicketManager;
use crate::{
    Result,
    TicketGuildManager,
    send_support_message,
    thread_name,
    to_title_case,
};

pub struct TicketModal;

impl TicketModal {
    pub async fn run<
        Db: Database,
        GuildManager: TicketGuildManager<Db>,
        Manager: TicketManager<Db>,
    >(
        http: &Http,
        interaction: &ModalInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        use zayden_core::Error as ZaydenError;

        let guild_id = interaction.guild_id.ok_or(ZaydenError::MissingGuildId)?;

        let guild_row = GuildManager::get(pool, guild_id)
            .await?
            .ok_or(crate::Error::SupportNotFound)?;

        let message = interaction
            .message
            .as_ref()
            .expect("modal interaction always has a message");
        let ticket_row = Manager::get(pool, message.id).await?;
        let role_ids = ticket_row.role_ids();

        let mut data =
            parse_modal_components(interaction.data.components.as_slice());
        let content = data
            .remove("ticket_body")
            .expect("Issue is a required field")
            .pop()
            .expect("At least one value is required");

        let thread_name = thread_name(
            guild_row.thread_id,
            interaction
                .member
                .as_ref()
                .expect("guild interaction always has a member")
                .display_name(),
            &content,
        );

        let mut issue = CreateEmbed::new().title("Issue").description(content);

        if let Some(mut version) = data.remove("version") {
            issue = issue.footer(CreateEmbedFooter::new(
                version.pop().expect("At least one value is required"),
            ));
        }

        let mut messages: Vec<CreateMessage<'_>> =
            vec![CreateMessage::new().embed(issue)];

        data.drain().filter(|(_, v)| !v.is_empty()).for_each(|(k, mut v)| {
            let title = to_title_case(&k);
            let embed = CreateEmbed::new()
                .title(title)
                .description(v.pop().expect("At least one value is required"));
            messages.push(CreateMessage::new().embed(embed));
        });

        let mut additional = data.remove("additional").unwrap_or_default();
        if !additional.is_empty() {
            let additional =
                CreateEmbed::new().title("Additional Information").description(
                    additional.pop().expect("At least one value is required"),
                );

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

        GuildManager::update_thread_id(pool, guild_id).await?;

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
