use std::fmt::Display;

use serenity::all::{
    CreateEmbed,
    CreateMessage,
    DiscordJsonError,
    EditMessage,
    ErrorResponse,
    Http,
    HttpError,
    JsonErrorCode,
    Mentionable,
    ThreadId,
    UserId,
};

use crate::Result;
use crate::templates::{Template, TemplateInfo};

pub async fn update_embeds<'a, T: Template>(
    http: &'a Http,
    row: &(impl TemplateInfo + Sync),
    owner_name: &str,
    thread: impl Into<ThreadId> + Send,
) -> Result<CreateEmbed<'a>> {
    let thread = thread.into();

    let embed = T::thread_embed(row, owner_name);

    if let (Some(channel), Some(message)) =
        (row.schedule_channel(), row.alt_message())
    {
        let embed = T::message_embed(row, owner_name, thread);

        match channel
            .edit_message(http, message, EditMessage::new().embed(embed))
            .await
        {
            Ok(_)
            | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                ErrorResponse {
                    error:
                        DiscordJsonError { code: JsonErrorCode::UnknownMessage, .. },
                    ..
                },
            ))) => {},
            Err(e) => return Err(e.into()),
        }
    }

    Ok(embed)
}

pub enum Announcement {
    Joined { user: UserId, alternative: bool },
    Left(UserId),
}

impl Announcement {
    pub async fn send(&self, http: &Http, channel: ThreadId) -> Result<()> {
        channel
            .widen()
            .send_message(http, CreateMessage::new().content(format!("{self}")))
            .await?;
        Ok(())
    }
}

impl Display for Announcement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Joined { user, alternative } if *alternative => {
                write!(f, "{} joined as an alternative", user.mention())
            },
            Self::Joined { user, .. } => {
                write!(f, "{} joined the fireteam", user.mention())
            },
            Self::Left(user) => {
                write!(f, "{} left the fireteam", user.mention())
            },
        }
    }
}
