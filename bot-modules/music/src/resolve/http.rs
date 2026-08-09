use std::time::Duration;

use songbird_reqwest::Client;
use zayden_app::services::http::HTTP_CONNECT_TIMEOUT;

use crate::error::{MusicError, Result};

pub const STREAM_READ_TIMEOUT: Duration = Duration::from_secs(20);

pub fn stream_client() -> Result<Client> {
    stream_client_with(HTTP_CONNECT_TIMEOUT, STREAM_READ_TIMEOUT)
}

pub fn stream_client_with(connect: Duration, read: Duration) -> Result<Client> {
    Client::builder().connect_timeout(connect).read_timeout(read).build().map_err(
        |e| {
            MusicError::Internal(format!(
                "could not build the audio streaming client: {e}"
            ))
        },
    )
}
