use hmac::{Hmac, KeyInit, Mac};
use reqwest::Client;
use sha1::Sha1;
use url::Url;

use crate::error::{Result, YoutubeError};

pub const HUB_URL: &str = "https://pubsubhubbub.appspot.com/subscribe";
pub const SIGNATURE_HEADER: &str = "x-hub-signature";
pub const CHANNEL_PARAM: &str = "channel";

const TOPIC_ROOT: &str = "https://www.youtube.com/xml/feeds/videos.xml";
pub const LEASE_SECONDS: i64 = 432_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Subscribe,
    Unsubscribe,
}

impl Mode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Subscribe => "subscribe",
            Self::Unsubscribe => "unsubscribe",
        }
    }

    #[must_use]
    pub fn from_param(mode: &str) -> Option<Self> {
        match mode {
            "subscribe" => Some(Self::Subscribe),
            "unsubscribe" => Some(Self::Unsubscribe),
            _ => None,
        }
    }
}

#[must_use]
pub fn topic_url(channel_id: &str) -> String {
    format!("{TOPIC_ROOT}?channel_id={channel_id}")
}

#[must_use]
pub fn channel_from_topic(topic: &str) -> Option<String> {
    let url = Url::parse(topic).ok()?;

    if url.host_str() != Some("www.youtube.com")
        || url.path() != "/xml/feeds/videos.xml"
    {
        return None;
    }

    url.query_pairs()
        .find(|(key, _)| key == "channel_id")
        .map(|(_, value)| value.into_owned())
}

pub fn callback_url(webhook_uri: &str, channel_id: &str) -> Result<String> {
    let mut url = Url::parse(webhook_uri)
        .map_err(|e| YoutubeError::Internal(e.to_string()))?;

    url.query_pairs_mut().append_pair(CHANNEL_PARAM, channel_id);

    Ok(url.into())
}

pub async fn request(
    client: &Client,
    mode: Mode,
    webhook_uri: &str,
    channel_id: &str,
    secret: &str,
) -> Result<()> {
    let callback = callback_url(webhook_uri, channel_id)?;
    let topic = topic_url(channel_id);
    let lease = LEASE_SECONDS.to_string();

    let mut form = vec![
        ("hub.callback", callback.as_str()),
        ("hub.mode", mode.as_str()),
        ("hub.topic", topic.as_str()),
        ("hub.verify", "async"),
    ];

    if mode == Mode::Subscribe {
        form.push(("hub.secret", secret));
        form.push(("hub.lease_seconds", &lease));
    }

    let response = client.post(HUB_URL).form(&form).send().await?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(YoutubeError::Hub(response.status().as_u16()))
    }
}

#[must_use]
pub fn verify(body: &[u8], signature: &str, secret: &str) -> bool {
    let Some(hex) = signature.strip_prefix("sha1=") else { return false };
    let Some(expected) = decode_hex(hex) else { return false };

    let Ok(mut mac) = Hmac::<Sha1>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);

    mac.verify_slice(&expected).is_ok()
}

fn decode_hex(input: &str) -> Option<Vec<u8>> {
    if !input.len().is_multiple_of(2) {
        return None;
    }

    input
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let hex = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(hex, 16).ok()
        })
        .collect()
}
