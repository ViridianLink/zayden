use std::fmt;

#[derive(Debug)]
pub enum AiError {
    Reqwest(reqwest::Error),
    ParseResponse { source: serde_json::Error, body: String },
    InvalidHeader(String),
    InvalidUrl(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reqwest(e) => write!(f, "reqwest: {e}"),
            Self::ParseResponse { source, .. } => {
                write!(f, "parse response: {source}")
            },
            Self::InvalidHeader(s) => write!(f, "invalid header: {s}"),
            Self::InvalidUrl(s) => write!(f, "invalid URL: {s}"),
        }
    }
}

impl std::error::Error for AiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Reqwest(e) => Some(e),
            Self::ParseResponse { source, .. } => Some(source),
            Self::InvalidHeader(_) | Self::InvalidUrl(_) => None,
        }
    }
}

impl From<reqwest::Error> for AiError {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}
