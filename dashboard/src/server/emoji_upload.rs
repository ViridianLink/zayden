use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use url::Url;

use crate::server::error::EmojiUploadError;

pub const MAX_EMOJI_BYTES: usize = 256 * 1024;
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);

#[must_use]
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.starts_with(b"RIFF")
        && bytes.get(8..12) == Some(b"WEBP".as_slice())
    {
        Some("image/webp")
    } else {
        None
    }
}

pub fn decode_data_uri(uri: &str) -> Result<Vec<u8>, EmojiUploadError> {
    let (header, payload) = uri
        .strip_prefix("data:")
        .and_then(|rest| rest.split_once(','))
        .ok_or(EmojiUploadError::BadDataUri)?;
    if !header.ends_with(";base64") {
        return Err(EmojiUploadError::BadDataUri);
    }
    if payload.len() > MAX_EMOJI_BYTES.div_ceil(3) * 4 {
        return Err(EmojiUploadError::TooLarge);
    }
    STANDARD.decode(payload).map_err(|_decode_error| EmojiUploadError::BadDataUri)
}

pub fn https_url(raw: &str) -> Result<Url, EmojiUploadError> {
    let url =
        Url::parse(raw.trim()).map_err(|_parse_error| EmojiUploadError::NotHttps)?;
    if url.scheme() == "https" && url.host_str().is_some() {
        Ok(url)
    } else {
        Err(EmojiUploadError::NotHttps)
    }
}

#[must_use]
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_multicast()
                || a == 0
                || (a == 100 && (64..128).contains(&b)))
        },
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80)
        },
    }
}

pub async fn fetch(url: &Url) -> Result<Vec<u8>, EmojiUploadError> {
    let host = url.host_str().ok_or(EmojiUploadError::NotHttps)?;
    let port = url.port_or_known_default().unwrap_or(443);
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| EmojiUploadError::Fetch(e.to_string()))?
        .collect();
    let pinned = *addrs
        .first()
        .ok_or_else(|| EmojiUploadError::Fetch(format!("{host} did not resolve")))?;
    if addrs.iter().any(|a| !is_public(a.ip())) {
        return Err(EmojiUploadError::PrivateAddress);
    }

    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .resolve(host, pinned)
        .build()
        .map_err(|e| EmojiUploadError::Fetch(e.to_string()))?;
    let mut response = client
        .get(url.clone())
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| EmojiUploadError::Fetch(e.to_string()))?;

    let mut bytes = Vec::new();
    while let Some(chunk) =
        response.chunk().await.map_err(|e| EmojiUploadError::Fetch(e.to_string()))?
    {
        if bytes.len() + chunk.len() > MAX_EMOJI_BYTES {
            return Err(EmojiUploadError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub fn data_uri(bytes: &[u8]) -> Result<String, EmojiUploadError> {
    if bytes.len() > MAX_EMOJI_BYTES {
        return Err(EmojiUploadError::TooLarge);
    }
    let mime = sniff(bytes).ok_or(EmojiUploadError::UnsupportedType)?;
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}
