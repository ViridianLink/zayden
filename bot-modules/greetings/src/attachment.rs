use reqwest::header::CONTENT_TYPE;
use serenity::all::CreateAttachment;

use crate::error::{GreetingsError, Result};
use crate::kind::GreetingKind;

pub const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;

const DEFAULT_EXTENSION: &str = "png";

pub async fn fetch(
    http: &reqwest::Client,
    url: &str,
    kind: GreetingKind,
) -> Result<CreateAttachment<'static>> {
    let response = http.get(url).send().await?.error_for_status()?;

    let too_large = || {
        GreetingsError::ImageUnusable(format!("larger than {MAX_IMAGE_BYTES} bytes"))
    };

    if response.content_length().is_some_and(|len| len > MAX_IMAGE_BYTES) {
        return Err(too_large());
    }

    let extension = extension_for(
        response.headers().get(CONTENT_TYPE).and_then(|value| value.to_str().ok()),
        url,
    )?;

    let bytes = response.bytes().await?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_IMAGE_BYTES {
        return Err(too_large());
    }

    Ok(CreateAttachment::bytes(bytes, format!("{kind}.{extension}"))
        .description(kind.image_alt()))
}

pub fn extension_for(content_type: Option<&str>, url: &str) -> Result<&'static str> {
    let mime = content_type.map(mime_essence);
    let mime = mime.as_deref();

    if let Some(extension) = mime.and_then(image_extension) {
        return Ok(extension);
    }

    if let Some(mime) = mime.filter(|mime| !is_opaque_binary(mime)) {
        return Err(GreetingsError::ImageUnusable(format!("served as {mime}")));
    }

    Ok(url_extension(url).unwrap_or(DEFAULT_EXTENSION))
}

fn mime_essence(raw: &str) -> String {
    raw.split(';').next().unwrap_or(raw).trim().to_ascii_lowercase()
}

fn is_opaque_binary(mime: &str) -> bool {
    matches!(mime, "application/octet-stream" | "binary/octet-stream")
}

fn image_extension(mime: &str) -> Option<&'static str> {
    match mime {
        "image/gif" => Some("gif"),
        "image/png" | "image/apng" => Some("png"),
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/avif" => Some("avif"),
        _ => None,
    }
}

fn url_extension(url: &str) -> Option<&'static str> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let segment = path.rsplit('/').next().unwrap_or(path);
    let (_, extension) = segment.rsplit_once('.')?;

    match extension.to_ascii_lowercase().as_str() {
        "gif" => Some("gif"),
        "png" | "apng" => Some("png"),
        "jpg" | "jpeg" => Some("jpg"),
        "webp" => Some("webp"),
        "avif" => Some("avif"),
        _ => None,
    }
}
