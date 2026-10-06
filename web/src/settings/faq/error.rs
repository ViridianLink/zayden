use crate::guild::GuildError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(in crate::settings) enum ArticleError {
    #[error(transparent)]
    Guild(#[from] GuildError),
    #[error("A title and a body are both required.")]
    TitleAndBodyRequired,
    #[error("That article no longer exists.")]
    NoSuchArticle,
}

impl From<sqlx::Error> for ArticleError {
    fn from(e: sqlx::Error) -> Self {
        Self::Guild(e.into())
    }
}

impl From<ArticleError> for GuildError {
    fn from(e: ArticleError) -> Self {
        match e {
            ArticleError::Guild(e) => e,
            ArticleError::TitleAndBodyRequired | ArticleError::NoSuchArticle => {
                Self::Server(e.to_string())
            },
        }
    }
}
