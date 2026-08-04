use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::CreateChatCompletionRequestArgs;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use zayden_app::services::http::ClientBuilderExt;

use crate::chat::Message;
use crate::error::AiError as Error;

const HTTP_REFERER: &str = "https://zayden.discord.bot";
const APP_TITLE: &str = "Zayden";

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

        let response = self.client.chat().create(request).await?;

        response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or(Error::NoContent)
    }
}
