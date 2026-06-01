use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage,
};

pub enum Role {
    System,
    User,
    Assistant,
}

pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self { role, content: content.into() }
    }
}

impl From<Message> for ChatCompletionRequestMessage {
    fn from(msg: Message) -> Self {
        match msg.role {
            Role::System => {
                ChatCompletionRequestSystemMessage::from(msg.content).into()
            },
            Role::User => ChatCompletionRequestUserMessage::from(msg.content).into(),
            Role::Assistant => {
                ChatCompletionRequestAssistantMessage::from(msg.content).into()
            },
        }
    }
}
