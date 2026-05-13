use std::fmt;

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    ParseResponse {
        source: serde_json::Error,
        body: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reqwest(e) => write!(f, "reqwest: {e}"),
            Self::ParseResponse { source, .. } => write!(f, "parse response: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Reqwest(e) => Some(e),
            Self::ParseResponse { source, .. } => Some(source),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}
