use topcoat::context::Cx;
use tracing::warn;
use twilight_model::id::Id;

use super::convert::reserved_keys;
use super::dto::{EmojiInfo, EmojiSource};
use super::emoji_upload;
use super::error::{AdminError, EmojiUploadError};
use super::keys::is_valid_key;
use crate::auth::{WebRole, app_state, discord_client, require_role};

pub async fn zayden_emojis(cx: &Cx) -> Vec<EmojiInfo> {
    let (http, app) = match (discord_client(cx), app_state(cx)) {
        (Ok(http), Ok(app)) => (http, app),
        (Err(e), _) | (_, Err(e)) => {
            warn!(error = %e, "cannot check Zayden's application emojis");
            return Vec::new();
        },
    };
    let Some(application) = Id::new_checked(app.zayden_id) else {
        warn!("cannot check Zayden's application emojis: zayden_id is 0");
        return Vec::new();
    };
    match http.get_application_emojis(application).await {
        Ok(response) => match response.model().await {
            Ok(list) => list
                .items
                .into_iter()
                .map(|e| EmojiInfo { name: e.name, id: e.id.to_string() })
                .collect(),
            Err(e) => {
                warn!(error = %e, "could not decode Zayden's application emojis");
                Vec::new()
            },
        },
        Err(e) => {
            warn!(error = %e, "could not list Zayden's application emojis");
            Vec::new()
        },
    }
}

pub async fn emoji_image(source: EmojiSource) -> Result<String, EmojiUploadError> {
    let bytes = match source {
        EmojiSource::Url(raw) => {
            emoji_upload::fetch(&emoji_upload::https_url(&raw)?).await?
        },
        EmojiSource::DataUri(uri) => emoji_upload::decode_data_uri(&uri)?,
    };
    emoji_upload::data_uri(&bytes)
}

pub async fn create_zayden_emoji(
    cx: &Cx,
    name: String,
    source: EmojiSource,
) -> Result<EmojiInfo, AdminError> {
    require_role(cx, WebRole::Admin).await?;

    if !is_valid_key(&name) {
        return Err(EmojiUploadError::InvalidName.into());
    }
    if reserved_keys().contains(&name) {
        return Err(EmojiUploadError::ReservedName(name).into());
    }
    if zayden_emojis(cx).await.iter().any(|e| e.name == name) {
        return Err(EmojiUploadError::NameTaken(name).into());
    }
    let image = emoji_image(source).await?;

    let http = discord_client(cx)?;
    let application = Id::new_checked(app_state(cx)?.zayden_id)
        .ok_or(AdminError::ZaydenIdNotConfigured)?;
    let emoji = http
        .add_application_emoji(application, &name, &image)
        .await
        .map_err(|e| EmojiUploadError::Discord(e.to_string()))?
        .model()
        .await
        .map_err(|e| EmojiUploadError::Discord(e.to_string()))?;

    Ok(EmojiInfo { name: emoji.name, id: emoji.id.to_string() })
}
