use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestMessage,
    ChatCompletionRequestMessageContentPartImage,
    ChatCompletionRequestMessageContentPartText,
    ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent,
    ChatCompletionRequestUserMessageContentPart,
    ImageUrl,
};

pub enum Role {
    System,
    User,
    Assistant,
}

pub struct Message {
    pub role: Role,
    pub content: String,
    pub images: Vec<String>,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self { role, content: content.into(), images: Vec::new() }
    }

    pub fn with_images(content: impl Into<String>, images: Vec<String>) -> Self {
        Self { role: Role::User, content: content.into(), images }
    }
}

impl From<Message> for ChatCompletionRequestMessage {
    fn from(msg: Message) -> Self {
        match msg.role {
            Role::System => {
                ChatCompletionRequestSystemMessage::from(msg.content).into()
            },
            Role::User if msg.images.is_empty() => {
                ChatCompletionRequestUserMessage::from(msg.content).into()
            },
            Role::User => ChatCompletionRequestUserMessage::from(
                ChatCompletionRequestUserMessageContent::from(user_parts(
                    msg.content,
                    msg.images,
                )),
            )
            .into(),
            Role::Assistant => {
                ChatCompletionRequestAssistantMessage::from(msg.content).into()
            },
        }
    }
}

fn user_parts(
    content: String,
    images: Vec<String>,
) -> Vec<ChatCompletionRequestUserMessageContentPart> {
    let text = (!content.is_empty())
        .then(|| ChatCompletionRequestMessageContentPartText::from(content).into());

    text.into_iter()
        .chain(images.into_iter().map(|url| {
            ChatCompletionRequestMessageContentPartImage::from(ImageUrl::from(url))
                .into()
        }))
        .collect()
}

#[must_use]
pub fn strip_speaker_prefix<'a>(reply: &'a str, speakers: &[&str]) -> &'a str {
    let mut stripped = reply.trim();

    while let Some(rest) =
        speakers.iter().find_map(|speaker| strip_one(stripped, speaker))
    {
        if rest.is_empty() {
            break;
        }

        stripped = rest;
    }

    stripped
}

fn strip_one<'a>(reply: &'a str, speaker: &str) -> Option<&'a str> {
    if speaker.is_empty() {
        return None;
    }

    let head = reply.get(..speaker.len())?;
    if !head.eq_ignore_ascii_case(speaker) {
        return None;
    }

    let rest = reply.get(speaker.len()..)?.strip_prefix(':')?;

    Some(rest.trim_start())
}
