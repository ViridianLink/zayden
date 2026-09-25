use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct YoutubeRuntime {
    pub api_key: Arc<str>,
    pub webhook_uri: Arc<str>,
}
