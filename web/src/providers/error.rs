use topcoat::router::StatusCode;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum FieldError {
    #[error("missing field `{0}`")]
    Missing(&'static str),
    #[error("duplicate field `{0}`")]
    Duplicate(&'static str),
    #[error("{key}: {reason}")]
    Invalid { key: &'static str, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum RequestRejection {
    #[error("Failed to deserialize query string: {0}")]
    Query(FieldError),
    #[error(
        "Form requests must have `Content-Type: application/x-www-form-urlencoded`"
    )]
    FormContentType,
    #[error("Failed to deserialize form body: {0}")]
    FormBody(FieldError),
}

impl RequestRejection {
    #[must_use]
    pub(super) const fn status(&self) -> StatusCode {
        match self {
            Self::Query(_) => StatusCode::BAD_REQUEST,
            Self::FormContentType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::FormBody(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}
