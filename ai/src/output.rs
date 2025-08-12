use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Response {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub status: String,
    pub background: bool,
    pub error: Option<serde_json::Value>,
    pub incomplete_details: Option<serde_json::Value>,
    pub instructions: String,
    pub max_output_tokens: i32,
    pub max_tool_calls: Option<serde_json::Value>,
    pub model: String,
    pub output: Vec<Output>,
    pub parallel_tool_calls: bool,
    pub previous_response_id: Option<serde_json::Value>,
    pub prompt_cache_key: Option<serde_json::Value>,
    pub reasoning: Reasoning,
    pub safety_identifier: Option<serde_json::Value>,
    pub service_tier: String,
    pub store: bool,
    pub temperature: f64,
    pub text: Text,
    pub tool_choice: String,
    pub tools: Vec<serde_json::Value>,
    pub top_logprobs: i32,
    pub top_p: f64,
    pub truncation: String,
    pub usage: Usage,
    pub user: Option<serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Output {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<Content>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Content {
    #[serde(rename = "type")]
    pub kind: String,
    pub annotations: Vec<serde_json::Value>,
    pub logprobs: Vec<serde_json::Value>,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct Reasoning {
    pub effort: String,
    pub summary: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Text {
    pub format: Format,
    pub verbosity: String,
}

#[derive(Debug, Deserialize)]
pub struct Format {
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: i32,
    pub input_tokens_details: InputTokensDetails,
    pub output_tokens: i32,
    pub output_tokens_details: OutputTokensDetails,
    pub total_tokens: i32,
}

#[derive(Debug, Deserialize)]
pub struct InputTokensDetails {
    pub cached_tokens: i32,
}

#[derive(Debug, Deserialize)]
pub struct OutputTokensDetails {
    pub reasoning_tokens: i32,
}
