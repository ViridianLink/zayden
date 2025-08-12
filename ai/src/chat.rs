use std::{collections::HashMap, num::NonZeroU64};

use serde::Serialize;

#[derive(Serialize)]
pub struct ResponseBody {
    background: bool,
    include: Vec<String>,
    pub input: Vec<Input>,
    pub instructions: Option<String>,
    pub max_output_tokens: Option<NonZeroU64>,
    max_tool_calls: Option<NonZeroU64>,
    metadata: Option<HashMap<String, String>>,
    pub model: String,
    parallel_tool_calls: Option<bool>,
    previous_response_id: Option<String>,
    prompt: Option<Prompt>,
    prompt_cache_key: Option<String>,
    reasoning: Reasoning,
    safety_identifier: Option<String>,
    service_tier: Option<ServiceTier>,
    store: Option<bool>,
    stream: Option<bool>,
    stream_options: Option<StreamOptions>,
    temperature: Option<f32>,
    text: Option<String>,
    tool_choice: Option<String>,
    tools: Option<Vec<String>>,
    top_logprobs: Option<u8>,
    top_p: Option<f32>,
    truncation: Option<String>,
    // verbosity: Option<String>,
}

impl ResponseBody {
    pub fn basic() -> Self {
        let reasoning = Reasoning {
            effort: Effort::Minimal,
            ..Default::default()
        };

        Self {
            background: false,
            include: Vec::new(),
            input: Vec::new(),
            instructions: None,
            max_output_tokens: Some(NonZeroU64::new(100).unwrap()),
            max_tool_calls: None,
            metadata: None,
            model: String::from("gpt-5-nano"),
            parallel_tool_calls: None,
            previous_response_id: None,
            prompt: None,
            prompt_cache_key: None,
            reasoning,
            safety_identifier: None,
            service_tier: None,
            store: None,
            stream: None,
            stream_options: None,
            temperature: None,
            text: None,
            tool_choice: None,
            tools: None,
            top_logprobs: None,
            top_p: None,
            truncation: None,
            // verbosity: None,
        }
    }

    pub fn input(mut self, input: Input) -> Self {
        self.input.push(input);
        self
    }

    pub fn instructions(mut self, message: impl Into<String>) -> Self {
        self.instructions = Some(message.into());
        self
    }
}

#[derive(Serialize)]
pub struct Input {
    pub content: String,
    pub role: Role,
    #[serde(rename = "type")]
    kind: String,
}

impl Input {
    pub fn new(content: impl Into<String>, role: Role) -> Self {
        Self {
            content: content.into(),
            role,
            kind: String::from("message"),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Developer,
}

#[derive(Serialize)]
pub struct Prompt {
    id: String,
    variables: HashMap<String, String>,
    version: Option<String>,
}

#[derive(Serialize)]
pub struct Reasoning {
    effort: Effort,
    summary: Option<Summary>,
}

impl Default for Reasoning {
    fn default() -> Self {
        Self {
            effort: Effort::Medium,
            summary: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Minimal,
    Low,
    Medium,
    High,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Summary {
    Auto,
    Concise,
    Detailed,
}

#[derive(Serialize)]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Priority,
}

#[derive(Serialize)]
pub struct StreamOptions {
    include_obfuscation: Option<bool>,
}
