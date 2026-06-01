use std::fmt;

use async_openai::error::OpenAIError;
use zayden_core::error::{HandlerError, Respond};

#[derive(Debug)]
pub enum AiError {
    OpenAI(OpenAIError),
    /// Custom `reqwest::Client` construction failed (TLS / DNS misconfiguration).
    Reqwest(reqwest::Error),
    /// The provider returned a response with no text content in any choice.
    NoContent,
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenAI(e) => write!(f, "AI client error: {e}"),
            Self::Reqwest(e) => write!(f, "HTTP client build error: {e}"),
            Self::NoContent => write!(f, "AI response contained no text"),
        }
    }
}

impl std::error::Error for AiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OpenAI(e) => Some(e),
            Self::Reqwest(e) => Some(e),
            Self::NoContent => None,
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
