use std::sync::Arc;
use std::{collections::HashMap, ops::Deref};

use base64::Engine;
use base64::engine::general_purpose;
// use futures::future;
use reqwest::ClientBuilder;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use serenity::all::{
    ApplicationId, Context, DiscordJsonError, Emoji, EmojiId, ErrorResponse, Guild, GuildId,
    HttpError, UserId,
};
use serenity::small_fixed_array::FixedString;

const ZAYDEN_ID: ApplicationId = ApplicationId::new(787490197943091211);

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
pub struct EmojiCache(HashMap<FixedString, EmojiId>);

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

    pub async fn new_from_parent(ctx: &Context, parent_token: &str) -> serenity::Result<Self> {
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
            .filter(|(name, _)| !emojis.contains_key(*name))
            .collect::<HashMap<_, _>>();

        for (name, id) in missing_emojis {
            let bytes = client
                .get(format!("https://cdn.discordapp.com/emojis/{id}.webp"))
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap();

            let base64 = general_purpose::STANDARD.encode(&bytes);

            match ctx
                .create_application_emoji(name, &format!("data:image/webp;base64,{base64}"))
                .await
            {
                Ok(emoji) => {
                    emojis.insert(emoji.name, emoji.id);
                }
                Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                    error: DiscordJsonError { errors, .. },
                    ..
                }))) if errors
                    .first()
                    .is_some_and(|e| e.code == "APPLICATION_EMOJI_NAME_ALREADY_TAKEN") => {}

                Err(e) => panic!("Unhandled error: {e:?}"),
            }
        }

        Ok(Self(emojis))
    }

    pub async fn upload(&mut self, ctx: &Context, parent_token: &str, name: &str) {
        let zayden_emojis = Self::parent_emojis(parent_token).await;

        let emoji_id = *zayden_emojis
            .get(name)
            .unwrap_or_else(|| panic!("Emoji '{name}' doesn't exist on Zayden"));

        let bytes = reqwest::get(format!("https://cdn.discordapp.com/emojis/{emoji_id}.webp"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        let base64 = general_purpose::STANDARD.encode(&bytes);

        match ctx
            .create_application_emoji(name, &format!("data:image/webp;base64,{base64}"))
            .await
        {
            Ok(emoji) => {
                self.0.insert(emoji.name, emoji.id);
            }
            // Emoji already uploaded
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                error: DiscordJsonError { errors, .. },
                ..
            }))) if errors
                .first()
                .is_some_and(|e| e.code == "APPLICATION_EMOJI_NAME_ALREADY_TAKEN") =>
            {
                self.0.insert(FixedString::from_str_trunc(name), emoji_id);
            }
            Err(e) => panic!("Unhandled Serenity error: {e:?}"),
        };
    }

    async fn parent_emojis(parent_token: &str) -> HashMap<FixedString, EmojiId> {
        #[derive(Deserialize)]
        struct ApplicationEmojis {
            items: Vec<Emoji>,
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(serenity::constants::USER_AGENT).unwrap(),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bot {parent_token}")).unwrap(),
        );

        let client = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();

        let emojis = client
            .get(format!(
                "https://discord.com/api/v10/applications/{ZAYDEN_ID}/emojis"
            ))
            .send()
            .await
            .unwrap()
            .json::<ApplicationEmojis>()
            .await
            .unwrap();

        // let emojis = serde_json::from_str::<ApplicationEmojis>(&text)
        //     .unwrap_or_else(|_| panic!("Failed to parse: {text}"));

        emojis
            .items
            .into_iter()
            .map(|emoji| (emoji.name, emoji.id))
            .collect()
    }

    pub fn emoji(&self, name: &str) -> EmojiResult<EmojiId> {
        self.get(name).copied().ok_or(name.to_string())
    }

    pub fn emoji_str(&self, name: &str) -> EmojiResult<String> {
        self.emoji(name).map(|id| format!("<:{name}:{id}>"))
    }
}

impl Deref for EmojiCache {
    type Target = HashMap<FixedString, EmojiId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
