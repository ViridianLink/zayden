use serenity::all::{
    ButtonStyle,
    CreateButton,
    CreateMessage,
    Http,
    Mention,
    ThreadId,
};

pub mod components;
pub mod error;
pub mod message_command;
pub mod modal;
pub mod slash_commands;
pub mod support_guild_manager;
pub mod ticket_manager;

pub use components::TicketComponent;
use error::Result;
pub use error::TicketError;
pub use message_command::SupportMessageCommand;
pub use modal::TicketModal;
pub use support_guild_manager::TicketGuildRow;
pub use ticket_manager::TicketRow;

pub struct Support;
pub struct Ticket;

#[must_use]
pub fn thread_name(thread_id: i32, author_name: &str, content: &str) -> String {
    format!("{thread_id} - {author_name} - {content}").chars().take(100).collect()
}

pub async fn send_support_message(
    http: &Http,
    thread_id: ThreadId,
    mentions: &[Mention],
    messages: Vec<CreateMessage<'_>>,
) -> Result<()> {
    let mentions = mentions.iter().map(ToString::to_string).collect::<String>();

    let button = CreateButton::new("support_close")
        .label("Close")
        .style(ButtonStyle::Primary);

    let thread_id = thread_id.widen();

    let len = messages.len();
    let mut mentions = Some(mentions);
    let mut button = Some(button);

    for (i, mut message) in messages.into_iter().enumerate() {
        if let Some(m) = mentions.take()
            && i == 0
        {
            message = message.content(m);
        }

        if let Some(b) = button.take()
            && i == len - 1
        {
            message = message.button(b);
        }

        thread_id.send_message(http, message).await?;
    }

    Ok(())
}

fn to_title_case(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>()
                    + chars.as_str().to_lowercase().as_str()
            })
        })
        .collect::<Vec<String>>()
        .join(" ")
}
