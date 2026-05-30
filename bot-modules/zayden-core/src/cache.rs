use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose;
// use futures::future;
use reqwest::ClientBuilder;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
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
use tracing::error;

const ZAYDEN_ID: ApplicationId = ApplicationId::new(787_490_197_943_091_211);

pub type EmojiResult<T> = Result<T, String>;

pub trait GuildMembersCache: Send + Sync + 'static {
    fn get(&self) -> &HashMap<GuildId, Vec<UserId>>;

    fn get_mut(&mut self) -> &mut HashMap<GuildId, Vec<UserId>>;

    fn guild_create(&mut self, guild: &Guild) {
        self.get_mut()
            .insert(guild.id, guild.members.iter().map(|x| x.user.id).collect());
    }
}

pub trait EmojiCacheData: Send + Sync + 'static {
    fn emojis(&self) -> Arc<EmojiCache>;

    fn emojis_mut(&mut self) -> Option<&mut EmojiCache>;
}

#[derive(Default)]
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
        // let iter = current_emojis
        //     .into_iter()
        //     .map(|emoji| ctx.delete_application_emoji(emoji.id));
        // future::join_all(iter).await;

        let client = reqwest::Client::new();

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
            error!(emoji = name, "EmojiCache::upload: emoji not found on Zayden");
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
        #[derive(Deserialize)]
        struct ApplicationEmojis {
            items: Vec<Emoji>,
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(serenity::constants::USER_AGENT),
        );

        let Ok(auth_header) = HeaderValue::from_str(&format!("Bot {parent_token}"))
        else {
            error!("EmojiCache::parent_emojis: invalid bot token for header");
            return HashMap::new();
        };
        headers.insert(AUTHORIZATION, auth_header);

        let client = match ClientBuilder::new().default_headers(headers).build() {
            Ok(c) => c,
            Err(e) => {
                error!(error = ?e, "EmojiCache::parent_emojis: client build failed");
                return HashMap::new();
            },
        };

        let emojis = match client
            .get(format!(
                "https://discord.com/api/v10/applications/{ZAYDEN_ID}/emojis"
            ))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                error!(error = ?e, "EmojiCache::parent_emojis: request failed");
                return HashMap::new();
            },
        };

        let emojis = match emojis.json::<ApplicationEmojis>().await {
            Ok(e) => e,
            Err(e) => {
                error!(error = ?e, "EmojiCache::parent_emojis: response parse failed");
                return HashMap::new();
            },
        };

        // let emojis = serde_json::from_str::<ApplicationEmojis>(&text)
        //     .unwrap_or_else(|_| panic!("Failed to parse: {text}"));

        emojis.items.into_iter().map(|emoji| (emoji.name, emoji.id)).collect()
    }

    pub fn emoji(&self, name: &str) -> EmojiResult<EmojiId> {
        self.get(name).copied().ok_or_else(|| name.to_string())
    }

    pub fn emoji_str(&self, name: &str) -> EmojiResult<String> {
        self.emoji(name).map(|id| format!("<:{name}:{id}>"))
    }
}

impl Deref for EmojiCache {
    type Target = HashMap<FixedString<u8>, EmojiId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
