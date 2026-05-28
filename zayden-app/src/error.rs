use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("missing required environment variable: {0}")]
    MissingEnvVar(String),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error("failed to parse config.toml: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("i/o error reading config file: {0}")]
    Io(#[from] std::io::Error),
}
