use std::time::Duration;

use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use zayden_core::retry::{RetryBudget, retry};

const MAX_ERROR_BODY_CHARS: usize = 400;
pub const TIMEOUT: Duration = Duration::from_secs(15);
pub const RETRY: RetryBudget = RetryBudget::new(3, Duration::from_millis(500));

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("{service} returned HTTP {status} for {what}{detail}")]
    Status {
        service: &'static str,
        what: String,
        status: StatusCode,
        detail: String,
    },
    #[error(
        "{service} sent a response for {what} that could not be decoded: {source}"
    )]
    Decode {
        service: &'static str,
        what: String,
        #[source]
        source: serde_json::Error,
    },
}

impl ApiError {
    #[must_use]
    pub fn status_code(&self) -> Option<StatusCode> {
        match self {
            Self::Status { status, .. } => Some(*status),
            Self::Reqwest(e) => e.status(),
            Self::Decode { .. } => None,
        }
    }

    #[must_use]
    pub fn is_not_found(&self) -> bool {
        self.status_code() == Some(StatusCode::NOT_FOUND)
    }

    #[must_use]
    pub fn is_unauthorized(&self) -> bool {
        matches!(
            self.status_code(),
            Some(StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN)
        )
    }
}

#[must_use]
pub fn is_transient(error: &ApiError) -> bool {
    match error {
        ApiError::Reqwest(e) => {
            e.is_timeout()
                || e.is_connect()
                || e.is_request()
                || e.status().is_some_and(|s| {
                    s.is_server_error() || s == StatusCode::TOO_MANY_REQUESTS
                })
        },
        ApiError::Status { status, .. } => {
            status.is_server_error() || *status == StatusCode::TOO_MANY_REQUESTS
        },
        ApiError::Decode { .. } => false,
    }
}

pub async fn api_ok(
    resp: Response,
    service: &'static str,
    what: &str,
) -> ApiResult<Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }

    let body = resp.text().await.unwrap_or_default();
    let trimmed = body.trim().chars().take(MAX_ERROR_BODY_CHARS).collect::<String>();
    let detail =
        if trimmed.is_empty() { String::new() } else { format!(": {trimmed}") };

    Err(ApiError::Status { service, what: what.to_owned(), status, detail })
}

pub async fn fetch_json<T, F>(
    service: &'static str,
    what: &str,
    build: F,
) -> ApiResult<T>
where
    T: DeserializeOwned,
    F: Fn() -> RequestBuilder + Send + Sync,
{
    let body = fetch_text(service, what, build).await?;

    serde_json::from_str(&body).map_err(|source| ApiError::Decode {
        service,
        what: what.to_owned(),
        source,
    })
}

pub async fn fetch_text<F>(
    service: &'static str,
    what: &str,
    build: F,
) -> ApiResult<String>
where
    F: Fn() -> RequestBuilder + Send + Sync,
{
    retry(RETRY, is_transient, || {
        let request = build().timeout(TIMEOUT);
        async move {
            let resp = api_ok(request.send().await?, service, what).await?;
            Ok(resp.text().await?)
        }
    })
    .await
}

pub async fn send_ok<F>(service: &'static str, what: &str, build: F) -> ApiResult<()>
where
    F: Fn() -> RequestBuilder + Send + Sync,
{
    retry(RETRY, is_transient, || {
        let request = build().timeout(TIMEOUT);
        async move {
            api_ok(request.send().await?, service, what).await?;
            Ok(())
        }
    })
    .await
}

#[must_use]
pub fn encode_query(params: &[(&str, &str)]) -> String {
    let mut query = String::new();

    for (key, value) in params {
        if !query.is_empty() {
            query.push('&');
        }
        encode_component(key, &mut query);
        query.push('=');
        encode_component(value, &mut query);
    }

    query
}

// RFC 3986 2.3: everything outside the unreserved set is percent-encoded, so a
// space arrives as %20. Jellyseerr reads its query strings as RFC 3986 rather
// than as a form body and takes the '+' of form encoding literally.
fn encode_component(raw: &str, query: &mut String) {
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
        {
            query.push(char::from(byte));
        } else {
            query.push('%');
            query.push(hex(byte >> 4));
            query.push(hex(byte & 0x0f));
        }
    }
}

const fn hex(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'A' + nibble - 10) as char,
    }
}

#[must_use]
pub fn trim_base_url(mut base_url: String) -> String {
    while base_url.ends_with('/') {
        base_url.pop();
    }
    base_url
}
