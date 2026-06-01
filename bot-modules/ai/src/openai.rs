use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::CreateChatCompletionRequestArgs;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::chat::Message;
use crate::error::AiError as Error;

/// Sent to `OpenRouter` so the API dashboard can identify the calling app.
const HTTP_REFERER: &str = "https://zayden.discord.bot";
const APP_TITLE: &str = "Zayden";

/// Thin wrapper around `async-openai`'s `Client` pre-configured for
/// `OpenRouter` (or any `OpenAI`-compatible endpoint).
///
/// Injects `http-referer` and `x-title` headers required by `OpenRouter` via
/// a custom `reqwest::Client` so every chat call carries them automatically.
#[derive(Debug)]
pub struct AiClient {
    client: Client<OpenAIConfig>,
    model: String,
}

impl AiClient {
    /// Build an `AiClient` for the given base URL, API key, and model.
    ///
    /// `endpoint` is the base URL of the provider (e.g.
    /// `https://openrouter.ai/api/v1`).  `model` is the model identifier
    /// passed verbatim in each request (e.g. `google/gemini-2.5-flash`).
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

        let http_client =
            reqwest::ClientBuilder::new().default_headers(headers).build()?;

        Ok(Self {
            client: Client::with_config(config).with_http_client(http_client),
            model: model.to_owned(),
        })
    }

    /// Send `messages` to the model and return the first choice's text.
    ///
    /// Returns `Err(AiError::NoContent)` if the response has no text choices.
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

        let response = self.client.chat().create(request).await?;

        response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or(Error::NoContent)
    }
}
