use serenity::all::{ButtonStyle, CreateButton, CreateMessage, Http, Mention, ThreadId};

pub mod components;
pub mod error;
pub mod message_command;
pub mod modal;
pub mod slash_commands;
pub mod support_guild_manager;
pub mod ticket_manager;

pub use components::TicketComponent;
pub use error::Error;
use error::Result;
pub use message_command::SupportMessageCommand;
pub use modal::TicketModal;
pub use support_guild_manager::TicketGuildManager;
pub use ticket_manager::TicketManager;

pub struct Support;
pub struct Ticket;

pub fn thread_name(thread_id: i32, author_name: &str, content: &str) -> String {
    format!("{thread_id} - {author_name} - {content}")
        .chars()
        .take(100)
        .collect()
}

pub async fn send_support_message<'a>(
    http: &Http,
    thread_id: ThreadId,
    mentions: &[Mention],
    mut messages: Vec<CreateMessage<'a>>,
) -> Result<()> {
    let mentions = mentions
        .iter()
        .map(|mention| mention.to_string())
        .collect::<String>();

    let button = CreateButton::new("support_close")
        .label("Close")
        .style(ButtonStyle::Primary);

    let thread_id = thread_id.widen();

    if messages.len() == 1 {
        thread_id
            .send_message(
                http,
                messages.pop().unwrap().content(mentions).button(button),
            )
            .await
            .unwrap();

        return Ok(());
    }

    let last_idx = messages.len() - 1;

    for (i, message) in messages.into_iter().enumerate() {
        if i == 0 {
            thread_id
                .send_message(http, message.content(mentions.clone()))
                .await
                .unwrap();

            continue;
        }

        if i == last_idx {
            thread_id
                .send_message(http, message.button(button.clone()))
                .await
                .unwrap();

            continue;
        }

        thread_id.send_message(http, message).await.unwrap();
    }

    Ok(())
}

fn to_title_case(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>()
                        + chars.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}
