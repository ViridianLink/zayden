use std::fmt::Display;

use serenity::all::{Colour, CreateEmbed};
use zayden_core::{EmojiCache, FormatNum};

use crate::error::Result;
use crate::{AppliedEffect, GamblingError, SHOP_ITEMS};

#[must_use]
pub fn effects_summary(emojis: &EmojiCache, effects: &[AppliedEffect]) -> String {
    if effects.is_empty() {
        return String::new();
    }

    let list = effects
        .iter()
        .map(|effect| {
            SHOP_ITEMS
                .get(effect.id)
                .and_then(|item| item.as_str(emojis).ok())
                .unwrap_or_else(|| effect.name.to_string())
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("\n\n**Effects:** {list}")
}

#[derive(Clone, Copy)]
pub enum Emoji {
    Str(&'static str),
    Id(&'static str),
    None,
}

pub struct GameResult {
    pub name: String,
    pub emoji: Emoji,
}

impl GameResult {
    #[must_use]
    pub const fn new(name: String, emoji: Emoji) -> Self {
        Self { name, emoji }
    }

    #[must_use]
    pub const fn new_with_str(name: String, emoji: &'static str) -> Self {
        Self { name, emoji: Emoji::Str(emoji) }
    }
}

impl GameResult {
    #[must_use]
    pub const fn new_with_id(name: String, emoji: &'static str) -> Self {
        Self { name, emoji: Emoji::Id(emoji) }
    }

    pub fn emoji(&self, emojis: &EmojiCache) -> Result<String> {
        match self.emoji {
            Emoji::Id(id) => emojis
                .emoji(id)
                .map(|emoji_id| format!("<:{}:{emoji_id}>", self.name))
                .map_err(|n| {
                    GamblingError::Internal(format!("emoji '{n}' not in cache"))
                }),
            Emoji::Str(emoji) => Ok(String::from(emoji)),
            Emoji::None => Ok(String::new()),
        }
    }
}

impl Display for GameResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PartialEq for GameResult {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "game embed requires all display parameters"
)]
pub fn game_embed<'a>(
    emojis: &EmojiCache,
    title: &'a str,
    prediction: impl Into<GameResult>,
    outcome_text: &str,
    outcome: impl Into<GameResult>,
    bet: i64,
    payout: i64,
    coins: i64,
    effects: &[AppliedEffect],
) -> Result<CreateEmbed<'a>> {
    let prediction: GameResult = prediction.into();
    let outcome: GameResult = outcome.into();

    let win = prediction == outcome;

    let result =
        format!("Payout: {} ({})", payout.format(), (payout - bet).format());

    let colour = if win { Colour::DARK_GREEN } else { Colour::RED };

    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let desc = format!(
        "Your bet: {} <:coin:{coin}>

        **You bet on:** {} ({prediction})
        **{outcome_text}:** {} ({outcome})

        {result}
        Your coins: {}{}",
        bet.format(),
        prediction.emoji(emojis)?,
        outcome.emoji(emojis)?,
        coins.format(),
        effects_summary(emojis, effects),
    );

    Ok(CreateEmbed::<'a>::new().title(title).description(desc).colour(colour))
}
