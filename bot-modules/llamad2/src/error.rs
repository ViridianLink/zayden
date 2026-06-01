use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, LlamaD2Error>;

#[derive(Debug, thiserror::Error)]
pub enum LlamaD2Error {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
}

impl Respond for LlamaD2Error {}
