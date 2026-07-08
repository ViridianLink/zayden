use std::time::{Duration, Instant};

use reqwest::Client;
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::error::{MarathonError, Result};

const CLEARANCE_TTL: Duration = Duration::from_mins(15);

#[derive(Debug, Clone)]
pub(super) struct Clearance {
    pub(super) cookie: String,
    pub(super) user_agent: String,
    minted_at: Instant,
}

impl Clearance {
    fn is_fresh(&self) -> bool {
        self.minted_at.elapsed() < CLEARANCE_TTL
    }
}

#[derive(Debug, Deserialize)]
struct FlareSolverrResponse {
    status: String,
    message: String,
    solution: Option<FlareSolverrSolution>,
}

#[derive(Debug, Deserialize)]
struct FlareSolverrSolution {
    #[serde(rename = "userAgent")]
    user_agent: String,
    cookies: Vec<FlareSolverrCookie>,
}

#[derive(Debug, Deserialize)]
struct FlareSolverrCookie {
    name: String,
    value: String,
}

pub(super) async fn ensure(
    client: &Client,
    flaresolverr_url: &str,
    cached: &Mutex<Option<Clearance>>,
) -> Result<Clearance> {
    {
        let guard = cached.lock().await;
        if let Some(c) = guard.as_ref()
            && c.is_fresh()
        {
            return Ok(c.clone());
        }
    }
    mint(client, flaresolverr_url, cached).await
}

pub(super) async fn mint(
    client: &Client,
    flaresolverr_url: &str,
    cached: &Mutex<Option<Clearance>>,
) -> Result<Clearance> {
    let mut guard = cached.lock().await;
    if let Some(c) = guard.as_ref()
        && c.is_fresh()
    {
        return Ok(c.clone());
    }

    let body = serde_json::json!({
        "cmd": "request.get",
        "url": "https://mobalytics.gg/marathon/weapons",
        "maxTimeout": 60_000,
    });
    let resp: FlareSolverrResponse =
        client.post(flaresolverr_url).json(&body).send().await?.json().await?;

    if resp.status != "ok" {
        return Err(MarathonError::FlareSolverr(resp.message));
    }
    let solution = resp.solution.ok_or_else(|| {
        MarathonError::FlareSolverr("missing solution".to_string())
    })?;
    let cookie = solution
        .cookies
        .iter()
        .find(|c| c.name == "cf_clearance")
        .map(|c| c.value.clone())
        .ok_or_else(|| {
            MarathonError::FlareSolverr(
                "no cf_clearance cookie in solution".to_string(),
            )
        })?;

    let fresh = Clearance {
        cookie,
        user_agent: solution.user_agent,
        minted_at: Instant::now(),
    };
    *guard = Some(fresh.clone());
    drop(guard);
    Ok(fresh)
}
