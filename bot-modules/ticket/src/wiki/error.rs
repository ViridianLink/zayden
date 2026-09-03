#[derive(Debug)]
pub enum WikiError {
    InvalidUrl(String, url::ParseError),
    UnsupportedScheme(String),
    PageNotFound(String),
    PageForbidden,
    SourceView(u16),
    GraphQl { status: u16, message: String },
    EmptyResponse,
    Http(reqwest::Error),
}

impl std::fmt::Display for WikiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl(url, e) => write!(f, "invalid wiki url {url}: {e}"),
            Self::UnsupportedScheme(scheme) => {
                write!(f, "unsupported wiki url scheme: {scheme}")
            },
            Self::PageNotFound(path) => write!(f, "no wiki page at {path}"),
            Self::PageForbidden => write!(
                f,
                "the wiki API key is not allowed to read page source; grant its \
                 group `manage:pages` (GraphQL) or `read:source` (source view)"
            ),
            Self::SourceView(status) => {
                write!(f, "wiki source view returned HTTP {status}")
            },
            Self::GraphQl { status, message } => {
                write!(f, "wiki graphql error (HTTP {status}): {message}")
            },
            Self::EmptyResponse => write!(f, "wiki returned no data and no errors"),
            Self::Http(e) => write!(f, "wiki request failed: {e}"),
        }
    }
}

impl std::error::Error for WikiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidUrl(_, e) => Some(e),
            Self::Http(e) => Some(e),
            Self::UnsupportedScheme(_)
            | Self::PageNotFound(_)
            | Self::PageForbidden
            | Self::SourceView(_)
            | Self::GraphQl { .. }
            | Self::EmptyResponse => None,
        }
    }
}

impl From<reqwest::Error> for WikiError {
    fn from(value: reqwest::Error) -> Self {
        Self::Http(value)
    }
}
