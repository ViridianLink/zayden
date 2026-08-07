use std::future::Future;
use std::time::Duration;

use serenity::all::{ErrorResponse, HttpError, StatusCode};
use tokio::time::sleep;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryBudget {
    pub attempts: u32,
    pub backoff: Duration,
}

impl RetryBudget {
    #[must_use]
    pub const fn new(attempts: u32, backoff: Duration) -> Self {
        Self { attempts, backoff }
    }
}

#[must_use]
pub fn status_is_transient(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
}

#[must_use]
pub fn is_transient(error: &serenity::Error) -> bool {
    let serenity::Error::Http(http) = error else {
        return false;
    };

    if let HttpError::UnsuccessfulRequest(ErrorResponse { status_code, .. }) = http {
        return status_is_transient(*status_code);
    }

    matches!(http, HttpError::Request(_))
}

pub async fn retry<T, E, F, Fut>(
    budget: RetryBudget,
    is_retryable: impl Fn(&E) -> bool,
    mut op: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut backoff = budget.backoff;
    let mut attempt: u32 = 1;

    loop {
        let error = match op().await {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };

        if attempt >= budget.attempts || !is_retryable(&error) {
            return Err(error);
        }

        sleep(backoff).await;
        backoff = backoff.saturating_mul(2);
        attempt += 1;
    }
}

pub async fn retry_transient<T, F, Fut>(
    budget: RetryBudget,
    op: F,
) -> serenity::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = serenity::Result<T>>,
{
    retry(budget, is_transient, op).await
}
