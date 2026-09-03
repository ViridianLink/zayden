use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct FaqArticleInfo {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) content: String,
    pub(crate) category: String,
    pub(crate) tags: String,
    pub(crate) generated: bool,
    pub(crate) source_thread_id: Option<String>,
    pub(crate) updated_at: String,
}
