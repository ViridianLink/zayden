use std::fmt::Display;

use serenity::all::{Colour, CreateEmbed};
use zayden_core::{EmojiCache, FormatNum};

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
    pub fn new(name: impl Into<String>, emoji: Emoji) -> Self {
        Self {
            name: name.into(),
            emoji,
        }
    }

    pub fn new_with_str(name: impl Into<String>, emoji: &'static str) -> Self {
        Self {
            name: name.into(),
            emoji: Emoji::Str(emoji),
        }
    }
}

impl GameResult {
    pub fn new_with_id(name: impl Into<String>, emoji: &'static str) -> Self {
        Self {
            name: name.into(),
            emoji: Emoji::Id(emoji),
        }
    }

    pub fn emoji(&self, emojis: &EmojiCache) -> String {
        match self.emoji {
            Emoji::Id(id) => format!("<:{}:{}>", self.name, emojis.emoji(id).unwrap()),
            Emoji::Str(emoji) => String::from(emoji),
            Emoji::None => String::new(),
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

#[allow(clippy::too_many_arguments)]
pub fn game_embed<'a>(
    emojis: &EmojiCache,
    title: &'a str,
    prediction: impl Into<GameResult>,
    outcome_text: &str,
    outcome: impl Into<GameResult>,
    bet: i64,
    payout: i64,
    coins: i64,
) -> CreateEmbed<'a> {
    let prediction: GameResult = prediction.into();
    let outcome: GameResult = outcome.into();

    let win = prediction == outcome;

    let result = format!("Payout: {} ({})", payout.format(), (payout - bet).format());

    let colour = if win { Colour::DARK_GREEN } else { Colour::RED };

    let desc = format!(
        "Your bet: {} <:coin:{}>
        
        **You bet on:** {} ({prediction})
        **{outcome_text}:** {} ({outcome})
        
        {result}
        Your coins: {}",
        bet.format(),
        emojis.emoji("heads").unwrap(),
        prediction.emoji(emojis),
        outcome.emoji(emojis),
        coins.format()
    );

    CreateEmbed::<'a>::new()
        .title(title)
        .description(desc)
        .colour(colour)
}
