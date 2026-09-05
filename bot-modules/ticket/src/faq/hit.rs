use crate::faq::article::FaqArticle;
use crate::wiki::SearchResult;

const LOCAL_PREFIX: &str = "local:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaqSource {
    Wiki,
    Local { id: i32 },
}

#[derive(Debug, Clone)]
pub struct FaqHit {
    pub title: String,
    pub description: String,
    pub path: String,
    pub source: FaqSource,
}

impl From<SearchResult> for FaqHit {
    fn from(result: SearchResult) -> Self {
        Self {
            title: result.title,
            description: result.description,
            path: result.path,
            source: FaqSource::Wiki,
        }
    }
}

impl From<&FaqArticle> for FaqHit {
    fn from(article: &FaqArticle) -> Self {
        Self {
            title: article.title.clone(),
            description: article.summary.clone(),
            path: format!("{LOCAL_PREFIX}{}", article.id),
            source: FaqSource::Local { id: article.id },
        }
    }
}
