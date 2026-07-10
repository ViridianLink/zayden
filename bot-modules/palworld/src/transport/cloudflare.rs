use reqwest::Client;
use serde::Deserialize;

use crate::error::{PalworldError, Result};

const MAX_TIMEOUT_MS: u32 = 60_000;
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                                   (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

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

pub(super) async fn fetch_html(
    client: &Client,
    flaresolverr_url: Option<&str>,
    target: &str,
) -> Result<String> {
    match flaresolverr_url {
        Some(url) => get_rendered(client, url, target).await,
        None => Ok(client
            .get(target)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?),
    }
}

async fn get_rendered(
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
        return Err(PalworldError::FlareSolverr(resp.message));
    }
    resp.solution
        .map(|s| s.response)
        .ok_or_else(|| PalworldError::FlareSolverr("missing solution".to_string()))
}
