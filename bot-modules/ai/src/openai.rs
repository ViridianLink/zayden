use std::time::Duration;

use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use async_openai::types::chat::{
    CreateChatCompletionRequest,
    CreateChatCompletionRequestArgs,
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use zayden_app::services::http::ClientBuilderExt;

use crate::chat::Message;
use crate::error::AiError as Error;

const HTTP_REFERER: &str = "https://zayden.discord.bot";
const APP_TITLE: &str = "Zayden";

const MAX_ATTEMPTS: u32 = 2;
const RETRY_BACKOFF: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub struct AiClient {
    client: Client<OpenAIConfig>,
    model: String,
}

impl AiClient {
    pub fn new(api_key: &str, endpoint: &str, model: &str) -> Result<Self, Error> {
        let config =
            OpenAIConfig::new().with_api_key(api_key).with_api_base(endpoint);

        let mut headers = HeaderMap::with_capacity(2);
        headers.insert(
            HeaderName::from_static("http-referer"),
            HeaderValue::from_static(HTTP_REFERER),
        );
        headers.insert(
            HeaderName::from_static("x-title"),
            HeaderValue::from_static(APP_TITLE),
        );

        let http_client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .with_timeouts()
            .build()?;

        Ok(Self {
            client: Client::with_config(config).with_http_client(http_client),
            model: model.to_owned(),
        })
    }

    pub async fn chat(
        &self,
        messages: Vec<Message>,
        max_tokens: u32,
    ) -> Result<String, Error> {
        let messages: Vec<_> = messages.into_iter().map(Into::into).collect();

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .max_tokens(max_tokens)
            .build()?;

        let mut attempt = 1;

        loop {
            let error = match self.send(request.clone()).await {
                Ok(content) => return Ok(content),
                Err(e) => e,
            };

            if attempt >= MAX_ATTEMPTS || !error.is_transient() {
                return Err(error);
            }

            tracing::warn!(
                %error,
                attempt,
                model = %self.model,
                "AI request failed; retrying"
            );

            tokio::time::sleep(RETRY_BACKOFF).await;
            attempt += 1;
        }
    }

    async fn send(
        &self,
        request: CreateChatCompletionRequest,
    ) -> Result<String, Error> {
        let response = self.client.chat().create(request).await.map_err(classify)?;

        response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or(Error::NoContent)
    }
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ProviderError,
}

#[derive(Deserialize)]
struct ProviderError {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    code: Option<serde_json::Value>,
}

fn classify(err: OpenAIError) -> Error {
    if let OpenAIError::JSONDeserialize(_, body) = &err
        && let Some(provider) = provider_error(body)
    {
        return provider;
    }

    Error::OpenAI(err)
}

fn provider_error(body: &str) -> Option<Error> {
    let envelope: ErrorEnvelope = serde_json::from_str(body.trim()).ok()?;

    Some(Error::Provider {
        code: envelope.error.code.as_ref().and_then(status_code),
        message: envelope
            .error
            .message
            .unwrap_or_else(|| String::from("no message given")),
    })
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
