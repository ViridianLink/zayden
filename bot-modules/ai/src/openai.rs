use std::time::Duration;

use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use async_openai::types::chat::{
    CreateChatCompletionRequest,
    CreateChatCompletionRequestArgs,
    FinishReason,
    ReasoningEffort,
    ResponseFormat,
    ResponseFormatJsonSchema,
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use zayden_app::services::http::ClientBuilderExt;

use crate::chat::Message;
use crate::error::AiError as Error;

const HTTP_REFERER: &str = "https://zayden.discord.bot";
const APP_TITLE: &str = "Zayden";

const MAX_ATTEMPTS: u32 = 2;
const RETRY_BACKOFF: Duration = Duration::from_secs(1);

const REASONING_EFFORT: ReasoningEffort = ReasoningEffort::Low;

#[derive(Debug)]
pub struct AiClient {
    client: Client<OpenAIConfig>,
    model: String,
}

impl AiClient {
    pub const ERROR_CONTENT_LIMIT: usize = 512;

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
        max_completion_tokens: u32,
        temperature: Option<f32>,
    ) -> Result<String, Error> {
        let request =
            self.build_request(messages, max_completion_tokens, temperature, None)?;
        let (content, _) = self.chat_with_retry(request).await?;

        Ok(content)
    }

    pub async fn chat_json<T: DeserializeOwned>(
        &self,
        messages: Vec<Message>,
        max_completion_tokens: u32,
        temperature: Option<f32>,
        schema_name: &str,
        schema: serde_json::Value,
    ) -> Result<T, Error> {
        let response_format = ResponseFormat::JsonSchema {
            json_schema: ResponseFormatJsonSchema {
                description: None,
                name: schema_name.to_owned(),
                schema,
                strict: Some(true),
            },
        };
        let request = self.build_request(
            messages,
            max_completion_tokens,
            temperature,
            Some(response_format),
        )?;
        let (content, finish_reason) = self.chat_with_retry(request).await?;

        if finish_reason == Some(FinishReason::Length) {
            return Err(Error::Truncated { content: truncate_content(&content) });
        }

        serde_json::from_str(&content).map_err(|source| Error::InvalidJson {
            source,
            content: truncate_content(&content),
        })
    }

    fn build_request(
        &self,
        messages: Vec<Message>,
        max_completion_tokens: u32,
        temperature: Option<f32>,
        response_format: Option<ResponseFormat>,
    ) -> Result<CreateChatCompletionRequest, Error> {
        let mut builder = CreateChatCompletionRequestArgs::default();
        builder
            .model(&self.model)
            .messages(messages.into_iter().map(Into::into).collect::<Vec<_>>())
            .max_completion_tokens(max_completion_tokens)
            .reasoning_effort(REASONING_EFFORT);

        if let Some(temperature) = temperature {
            builder.temperature(temperature);
        }

        if let Some(response_format) = response_format {
            builder.response_format(response_format);
        }

        Ok(builder.build()?)
    }

    async fn chat_with_retry(
        &self,
        request: CreateChatCompletionRequest,
    ) -> Result<(String, Option<FinishReason>), Error> {
        let mut attempt = 1;

        loop {
            let error = match self.send(request.clone()).await {
                Ok(response) => return Ok(response),
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
    ) -> Result<(String, Option<FinishReason>), Error> {
        let response = self.client.chat().create(request).await.map_err(classify)?;

        response
            .choices
            .into_iter()
            .next()
            .and_then(|c| Some((c.message.content?, c.finish_reason)))
            .ok_or(Error::NoContent)
    }
}

fn truncate_content(content: &str) -> String {
    match content.char_indices().nth(AiClient::ERROR_CONTENT_LIMIT) {
        Some((end, _)) => {
            let mut truncated = String::with_capacity(end + 1);
            truncated.push_str(content.get(..end).unwrap_or_default());
            truncated.push('…');
            truncated
        },
        None => content.to_owned(),
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
