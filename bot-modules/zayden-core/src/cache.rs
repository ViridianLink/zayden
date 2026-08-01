use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose;
use dashmap::DashMap;
use reqwest::header::{
    AUTHORIZATION,
    HeaderMap,
    HeaderValue,
    RETRY_AFTER,
    USER_AGENT,
};
use reqwest::{Client, ClientBuilder, StatusCode};
use serde::Deserialize;
use serenity::all::{
    ApplicationId,
    Context,
    DataUri,
    DiscordJsonError,
    Emoji,
    EmojiId,
    ErrorResponse,
    Guild,
    GuildId,
    HttpError,
    UserId,
};
use serenity::small_fixed_array::FixedString;
use tracing::{error, warn};

const ZAYDEN_ID: ApplicationId = ApplicationId::new(787_490_197_943_091_211);
const PARENT_EMOJI_ATTEMPTS: u32 = 3;
const PARENT_EMOJI_BACKOFF: Duration = Duration::from_secs(1);
const PARENT_EMOJI_MAX_WAIT: Duration = Duration::from_secs(30);
const LOGGED_BODY_LIMIT: usize = 512;

pub type EmojiResult<T> = Result<T, String>;

pub trait GuildMembersCache: Send + Sync + 'static {
    fn get(&self) -> &DashMap<GuildId, Vec<UserId>>;

    fn guild_create(&self, guild: &Guild) {
        self.get()
            .insert(guild.id, guild.members.iter().map(|x| x.user.id).collect());
    }
}

pub trait EmojiCacheData: Send + Sync + 'static {
    fn emojis(&self) -> Arc<EmojiCache>;

    fn emojis_mut(&mut self) -> &mut EmojiCache;
}

#[derive(Clone, Default)]
pub struct EmojiCache(HashMap<FixedString<u8>, EmojiId>);

impl EmojiCache {
    pub async fn new(ctx: &Context) -> serenity::Result<Self> {
        Ok(Self(
            ctx.get_application_emojis()
                .await?
                .into_iter()
                .map(|emoji| (emoji.name, emoji.id))
                .collect(),
        ))
    }

    pub async fn new_from_parent(
        ctx: &Context,
        parent_token: &str,
    ) -> serenity::Result<Self> {
        let current_emojis = ctx.get_application_emojis().await?;

        let client = Client::new();

        let parent_emojis = Self::parent_emojis(parent_token).await;

        let mut emojis = current_emojis
            .into_iter()
            .map(|emoji| (emoji.name, emoji.id))
            .collect::<HashMap<_, _>>();

        let missing_emojis = parent_emojis
            .iter()
            .filter(|(name, _)| !emojis.contains_key(name.as_str()))
            .collect::<HashMap<_, _>>();

        for (name, id) in missing_emojis {
            let bytes = client
                .get(format!("https://cdn.discordapp.com/emojis/{id}.webp"))
                .send()
                .await?
                .bytes()
                .await?;

            let base64 = general_purpose::STANDARD.encode(&bytes);

            match ctx
                .create_application_emoji(name, {
                    let Ok(uri) = DataUri::from_base64(format!(
                        "data:image/webp;base64,{base64}"
                    )) else {
                        continue;
                    };

                    uri
                })
                .await
            {
                Ok(emoji) => {
                    emojis.insert(emoji.name, emoji.id);
                },
                Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                    ErrorResponse { error: DiscordJsonError { errors, .. }, .. },
                ))) if errors.first().is_some_and(|e| {
                    e.code == "APPLICATION_EMOJI_NAME_ALREADY_TAKEN"
                }) => {},

                Err(e) => return Err(e),
            }
        }

        Ok(Self(emojis))
    }

    pub async fn upload(&mut self, ctx: &Context, parent_token: &str, name: &str) {
        let zayden_emojis = Self::parent_emojis(parent_token).await;

        let Some(&emoji_id) = zayden_emojis.get(name) else {
            warn!(emoji = name, "EmojiCache::upload: emoji not found on Zayden");
            return;
        };

        let Ok(resp) = reqwest::get(format!(
            "https://cdn.discordapp.com/emojis/{emoji_id}.webp"
        ))
        .await
        else {
            error!(emoji = name, "EmojiCache::upload: CDN request failed");
            return;
        };
        let Ok(bytes) = resp.bytes().await else {
            error!(emoji = name, "EmojiCache::upload: CDN response failed");
            return;
        };

        let base64 = general_purpose::STANDARD.encode(&bytes);

        match ctx
            .create_application_emoji(name, {
                let Ok(uri) =
                    DataUri::from_base64(format!("data:image/webp;base64,{base64}"))
                else {
                    error!(emoji = name, "EmojiCache::upload: invalid base64");
                    return;
                };

                uri
            })
            .await
        {
            Ok(emoji) => {
                self.0.insert(emoji.name, emoji.id);
            },
            // Emoji already uploaded
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                ErrorResponse { error: DiscordJsonError { errors, .. }, .. },
            ))) if errors.first().is_some_and(|e| {
                e.code == "APPLICATION_EMOJI_NAME_ALREADY_TAKEN"
            }) =>
            {
                self.0.insert(FixedString::from_str_trunc(name), emoji_id);
            },
            Err(e) => error!(
                error = ?e,
                emoji = name,
                "EmojiCache::upload: failed to create application emoji",
            ),
        }
    }

    async fn parent_emojis(parent_token: &str) -> HashMap<FixedString<u8>, EmojiId> {
        let Some(client) = parent_client(parent_token) else {
            return HashMap::new();
        };

        let url =
            format!("https://discord.com/api/v10/applications/{ZAYDEN_ID}/emojis");

        let mut backoff = PARENT_EMOJI_BACKOFF;

        for attempt in 1..=PARENT_EMOJI_ATTEMPTS {
            match fetch_parent_emojis(&client, &url).await {
                FetchOutcome::Ok(emojis) => return emojis,
                FetchOutcome::Fatal => break,
                FetchOutcome::Retry(retry_after) => {
                    if attempt == PARENT_EMOJI_ATTEMPTS {
                        error!(
                            attempts = attempt,
                            "EmojiCache::parent_emojis: giving up, no parent emojis",
                        );
                        break;
                    }

                    let delay = retry_after.unwrap_or(backoff);
                    warn!(
                        attempt,
                        ?delay,
                        "EmojiCache::parent_emojis: retrying after transient failure",
                    );
                    tokio::time::sleep(delay).await;
                    backoff = backoff.saturating_mul(2);
                },
            }
        }

        HashMap::new()
    }

    pub fn emoji(&self, name: &str) -> EmojiResult<EmojiId> {
        self.get(name).copied().ok_or_else(|| name.to_string())
    }

    pub fn emoji_str(&self, name: &str) -> EmojiResult<String> {
        self.emoji(name).map(|id| format!("<:{name}:{id}>"))
    }

    pub fn merge_from(&mut self, other: &Self) {
        for (name, &id) in other.iter() {
            self.0.entry(name.clone()).or_insert(id);
        }
    }
}

impl Deref for EmojiCache {
    type Target = HashMap<FixedString<u8>, EmojiId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

enum FetchOutcome {
    Ok(HashMap<FixedString<u8>, EmojiId>),
    Retry(Option<Duration>),
    Fatal,
}

fn parent_client(parent_token: &str) -> Option<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(serenity::http::SERENITY_USER_AGENT),
    );

    let Ok(auth_header) = HeaderValue::from_str(&format!("Bot {parent_token}"))
    else {
        error!("EmojiCache::parent_emojis: invalid bot token for header");
        return None;
    };
    headers.insert(AUTHORIZATION, auth_header);

    match ClientBuilder::new().default_headers(headers).build() {
        Ok(client) => Some(client),
        Err(e) => {
            error!(error = ?e, "EmojiCache::parent_emojis: client build failed");
            None
        },
    }
}

async fn fetch_parent_emojis(client: &Client, url: &str) -> FetchOutcome {
    #[derive(Deserialize)]
    struct ApplicationEmojis {
        items: Vec<Emoji>,
    }

    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(e) => {
            warn!(error = ?e, "EmojiCache::parent_emojis: request failed");
            return FetchOutcome::Retry(None);
        },
    };

    let status = response.status();
    let retry_after = retry_after(&response);

    let body = match response.text().await {
        Ok(body) => body,
        Err(e) => {
            warn!(
                error = ?e,
                %status,
                "EmojiCache::parent_emojis: response read failed",
            );
            return FetchOutcome::Retry(None);
        },
    };

    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
        warn!(
            %status,
            body = logged_body(&body),
            "EmojiCache::parent_emojis: transient response",
        );
        return FetchOutcome::Retry(retry_after);
    }

    if !status.is_success() {
        error!(
            %status,
            body = logged_body(&body),
            "EmojiCache::parent_emojis: request rejected",
        );
        return FetchOutcome::Fatal;
    }

    match serde_json::from_str::<ApplicationEmojis>(&body) {
        Ok(emojis) => FetchOutcome::Ok(
            emojis.items.into_iter().map(|emoji| (emoji.name, emoji.id)).collect(),
        ),
        Err(e) => {
            error!(
                error = ?e,
                body = logged_body(&body),
                "EmojiCache::parent_emojis: response parse failed",
            );
            FetchOutcome::Fatal
        },
    }
}

fn retry_after(response: &reqwest::Response) -> Option<Duration> {
    let seconds =
        response.headers().get(RETRY_AFTER)?.to_str().ok()?.parse::<f64>().ok()?;

    Duration::try_from_secs_f64(seconds).ok().map(|d| d.min(PARENT_EMOJI_MAX_WAIT))
}

fn logged_body(body: &str) -> &str {
    match body.char_indices().nth(LOGGED_BODY_LIMIT) {
        Some((idx, _)) => body.get(..idx).unwrap_or(body),
        None => body,
    }
}
