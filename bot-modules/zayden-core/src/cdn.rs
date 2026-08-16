use serde::Deserialize;
use tracing::warn;

const REFRESH_ENDPOINT: &str =
    "https://discord.com/api/v10/attachments/refresh-urls";

const CDN_HOSTS: [&str; 2] = ["cdn.discordapp.com", "media.discordapp.net"];

#[derive(Deserialize)]
struct RefreshedUrls {
    refreshed_urls: Vec<RefreshedUrl>,
}

#[derive(Deserialize)]
struct RefreshedUrl {
    refreshed: String,
}

#[must_use]
pub fn is_discord_cdn(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split(['/', ':', '?', '#']).next().unwrap_or(rest);

    CDN_HOSTS.contains(&host.to_ascii_lowercase().as_str())
}

pub async fn refresh_attachment_url(
    http: &reqwest::Client,
    token: &str,
    url: &str,
) -> String {
    if !is_discord_cdn(url) {
        return url.to_string();
    }

    let response = http
        .post(REFRESH_ENDPOINT)
        .header(reqwest::header::AUTHORIZATION, format!("Bot {token}"))
        .json(&serde_json::json!({ "attachment_urls": [url] }))
        .send()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(e) => {
            warn!(error = ?e, "attachment refresh request failed");
            return url.to_string();
        },
    };

    let status = response.status();
    if !status.is_success() {
        warn!(%status, "attachment refresh rejected");
        return url.to_string();
    }

    match response.json::<RefreshedUrls>().await {
        Ok(body) => body
            .refreshed_urls
            .into_iter()
            .next()
            .map_or_else(|| url.to_string(), |first| first.refreshed),
        Err(e) => {
            warn!(error = ?e, "attachment refresh response was not understood");
            url.to_string()
        },
    }
}
