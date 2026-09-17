use serde::Deserialize;

use crate::error::AiError as Error;

#[derive(Debug, Deserialize)]
pub struct ChatCompletion {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    #[serde(default)]
    message: Option<ChoiceMessage>,
    #[serde(default)]
    finish_reason: Option<String>,
    #[serde(default)]
    error: Option<ProviderError>,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderError {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    code: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorEnvelope {
    error: ProviderError,
}

#[derive(Debug)]
pub struct Completion {
    pub content: String,
    pub truncated: bool,
}

const LENGTH_CAPPED: &str = "length";

impl ChatCompletion {
    pub fn into_completion(self) -> Result<Completion, Error> {
        let choice = self.choices.into_iter().next().ok_or(Error::NoContent)?;

        if let Some(error) = choice.error {
            return Err(error.into_error());
        }

        let content =
            choice.message.and_then(|m| m.content).ok_or(Error::NoContent)?;

        Ok(Completion {
            content,
            truncated: choice.finish_reason.as_deref() == Some(LENGTH_CAPPED),
        })
    }
}

impl ProviderError {
    fn into_error(self) -> Error {
        Error::Provider {
            code: self.code.as_ref().and_then(status_code),
            message: self
                .message
                .unwrap_or_else(|| String::from("no message given")),
        }
    }
}

impl ErrorEnvelope {
    #[must_use]
    pub fn into_error(self) -> Error {
        self.error.into_error()
    }
}

fn status_code(code: &serde_json::Value) -> Option<u16> {
    match code {
        serde_json::Value::Number(n) => {
            n.as_u64().and_then(|n| u16::try_from(n).ok())
        },
        serde_json::Value::String(s) => s.parse().ok(),
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Array(_)
        | serde_json::Value::Object(_) => None,
    }
}
