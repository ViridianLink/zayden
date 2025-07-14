use std::fmt::Display;

use serenity::all::{
    CreateMessage, DiscordJsonError, EditMessage, ErrorResponse, Http, HttpError, JsonErrorCode,
    Mentionable, ThreadId, UserId,
};

use crate::templates::{Template, TemplateInfo};

pub async fn update_embeds<T: Template>(
    http: &Http,
    row: &impl TemplateInfo,
    owner_name: &str,
    thread: impl Into<ThreadId>,
) {
    let thread = thread.into();

    let embed = T::thread_embed(row, owner_name);

    thread
        .widen()
        .edit_message(http, thread.get().into(), EditMessage::new().embed(embed))
        .await
        .unwrap();

    if let (Some(channel), Some(message)) = (row.schedule_channel(), row.alt_message()) {
        let embed = T::message_embed(row, owner_name, thread);

        match channel
            .edit_message(http, message, EditMessage::new().embed(embed))
            .await
        {
            Ok(_)
            | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                error:
                    DiscordJsonError {
                        code: JsonErrorCode::UnknownMessage,
                        ..
                    },
                ..
            }))) => {}
            Err(e) => panic!("{e:?}"),
        };
    }
}

pub enum Announcement {
    Joined { user: UserId, alternative: bool },
    Left(UserId),
}

impl Announcement {
    pub async fn send(&self, http: &Http, channel: ThreadId) {
        channel
            .widen()
            .send_message(http, CreateMessage::new().content(format!("{self}")))
            .await
            .unwrap();
    }
}

impl Display for Announcement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Announcement::Joined { user, alternative } if *alternative => {
                write!(f, "{} joined as an alternative", user.mention())
            }
            Announcement::Joined { user, .. } => {
                write!(f, "{} joined the fireteam", user.mention())
            }
            Announcement::Left(user) => write!(f, "{} left the fireteam", user.mention()),
        }
    }
}
