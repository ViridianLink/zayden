use std::{collections::HashMap, ops::Deref};

use base64::Engine;
use base64::engine::general_purpose;
use reqwest::ClientBuilder;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use serenity::all::{ApplicationId, Context, Emoji, EmojiId, Guild, GuildId, UserId};
use serenity::small_fixed_array::FixedString;
use tokio::sync::OnceCell;

const ZAYDEN_ID: ApplicationId = ApplicationId::new(787490197943091211);
static ZAYDEN_EMOJIS: OnceCell<HashMap<FixedString, EmojiId>> = OnceCell::const_new();

pub trait GuildMembersCache: Send + Sync + 'static {
    fn get(&self) -> &HashMap<GuildId, Vec<UserId>>;

    fn get_mut(&mut self) -> &mut HashMap<GuildId, Vec<UserId>>;

    fn guild_create(&mut self, guild: &Guild) {
        self.get_mut()
            .insert(guild.id, guild.members.iter().map(|x| x.user.id).collect());
    }
}

pub trait EmojiCacheData: Send + Sync + 'static {
    fn get(&self) -> &EmojiCache;

    fn get_mut(&mut self) -> &mut EmojiCache;
}

pub type EmojiResult<T> = Result<T, String>;

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

    pub async fn upload(&mut self, ctx: &Context, name: &str) {
        let zayden_emojis = ZAYDEN_EMOJIS
            .get_or_init(|| async {
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
                    HeaderValue::from_str(&format!(
                        "Bot {}",
                        std::env::var("DISCORD_TOKEN").unwrap()
                    ))
                    .unwrap(),
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

                emojis
                    .items
                    .into_iter()
                    .map(|emoji| (emoji.name, emoji.id))
                    .collect()
            })
            .await;

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

        let emoji = ctx
            .create_application_emoji(name, &format!("data:image/webp;base64,{base64}"))
            .await
            .unwrap();

        self.0.insert(emoji.name, emoji.id);
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
