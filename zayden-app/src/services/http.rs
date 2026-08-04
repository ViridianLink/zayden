use std::time::Duration;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
pub const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub trait ClientBuilderExt {
    #[must_use = "the budget applies to the returned builder, not to this one"]
    fn with_timeouts(self) -> Self;
}

impl ClientBuilderExt for reqwest::ClientBuilder {
    fn with_timeouts(self) -> Self {
        self.timeout(HTTP_TIMEOUT).connect_timeout(HTTP_CONNECT_TIMEOUT)
    }
}
