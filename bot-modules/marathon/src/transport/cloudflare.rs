use reqwest::Client;
use serde::Deserialize;

use crate::error::{MarathonError, Result};

const MAX_TIMEOUT_MS: u32 = 60_000;

#[derive(Debug, Deserialize)]
struct FlareSolverrResponse {
    status: String,
    message: String,
    solution: Option<FlareSolverrSolution>,
}

#[derive(Debug, Deserialize)]
struct FlareSolverrSolution {
    response: String,
}

pub(super) async fn get_rendered(
    client: &Client,
    flaresolverr_url: &str,
    target: &str,
) -> Result<String> {
    let body = serde_json::json!({
        "cmd": "request.get",
        "url": target,
        "maxTimeout": MAX_TIMEOUT_MS,
    });
    let resp: FlareSolverrResponse =
        client.post(flaresolverr_url).json(&body).send().await?.json().await?;

    if resp.status != "ok" {
        return Err(MarathonError::FlareSolverr(resp.message));
    }
    resp.solution
        .map(|s| s.response)
        .ok_or_else(|| MarathonError::FlareSolverr("missing solution".to_string()))
}
