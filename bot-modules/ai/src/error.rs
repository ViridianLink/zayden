use std::fmt;

use async_openai::error::OpenAIError;
use zayden_core::error::{HandlerError, Respond};

#[derive(Debug)]
pub enum AiError {
    OpenAI(OpenAIError),
    Provider { code: Option<u16>, message: String },
    Reqwest(reqwest::Error),
    NoContent,
    Truncated { content: String },
    InvalidJson { source: serde_json::Error, content: String },
}

impl AiError {
    #[must_use]
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Provider { code, .. } => code.is_some_and(is_transient_status),
            Self::OpenAI(e) => openai_is_transient(e),
            Self::Reqwest(_)
            | Self::NoContent
            | Self::Truncated { .. }
            | Self::InvalidJson { .. } => false,
        }
    }
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenAI(e) => write!(f, "AI client error: {e}"),
            Self::Provider { code: Some(code), message } => {
                write!(f, "AI provider error ({code}): {message}")
            },
            Self::Provider { code: None, message } => {
                write!(f, "AI provider error: {message}")
            },
            Self::Reqwest(e) => write!(f, "HTTP client build error: {e}"),
            Self::NoContent => write!(f, "AI response contained no text"),
            Self::Truncated { content } => write!(
                f,
                "AI response hit the token limit before finishing; \
partial content: {content}"
            ),
            Self::InvalidJson { source, content } => {
                write!(
                    f,
                    "AI response was not valid JSON: {source}; content: {content}"
                )
            },
        }
    }
}

impl std::error::Error for AiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OpenAI(e) => Some(e),
            Self::Reqwest(e) => Some(e),
            Self::InvalidJson { source, .. } => Some(source),
            Self::Provider { .. } | Self::NoContent | Self::Truncated { .. } => None,
        }
    }
}

impl From<OpenAIError> for AiError {
    fn from(e: OpenAIError) -> Self {
        Self::OpenAI(e)
    }
}

impl From<reqwest::Error> for AiError {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

impl Respond for AiError {}

impl From<AiError> for HandlerError {
    fn from(e: AiError) -> Self {
        Self::from_respond(e)
    }
}

const fn is_transient_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500..=599)
}

fn openai_is_transient(err: &OpenAIError) -> bool {
    if let OpenAIError::ApiError(e) = err {
        return is_transient_status(e.status_code.as_u16());
    }

    if let OpenAIError::Reqwest(e) = err {
        return e.is_timeout() || e.is_connect();
    }

    false
}
