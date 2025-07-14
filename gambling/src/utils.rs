use std::fmt::Display;

use serenity::all::{Colour, CreateEmbed, EmojiId};
use zayden_core::FormatNum;

use crate::COIN;

#[derive(Clone, Copy)]
pub enum Emoji<'a> {
    Str(&'a str),
    Id(EmojiId),
    None,
}

pub struct GameResult<'a> {
    pub name: String,
    pub emoji: Emoji<'a>,
}

impl<'a> GameResult<'a> {
    pub fn new(name: impl Into<String>, emoji: Emoji<'a>) -> Self {
        Self {
            name: name.into(),
            emoji,
        }
    }

    pub fn new_with_str(name: impl Into<String>, emoji: &'a str) -> Self {
        Self {
            name: name.into(),
            emoji: Emoji::Str(emoji),
        }
    }
}

impl GameResult<'_> {
    pub fn new_with_id(name: impl Into<String>, emoji: EmojiId) -> Self {
        Self {
            name: name.into(),
            emoji: Emoji::Id(emoji),
        }
    }

    pub fn emoji(&self) -> String {
        match self.emoji {
            Emoji::Id(id) => format!("<:{}:{id}>", self.name),
            Emoji::Str(emoji) => String::from(emoji),
            Emoji::None => String::new(),
        }
    }
}

impl Display for GameResult<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PartialEq for GameResult<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

pub fn game_embed<'a>(
    title: &'a str,
    prediction: impl Into<GameResult<'a>>,
    outcome_text: &str,
    outcome: impl Into<GameResult<'a>>,
    bet: i64,
    payout: i64,
    coins: i64,
) -> CreateEmbed<'a> {
    let prediction = prediction.into();
    let outcome = outcome.into();

    let win = prediction == outcome;

    let result = format!("Payout: {} ({})", payout.format(), (payout - bet).format());

    let colour = if win { Colour::DARK_GREEN } else { Colour::RED };

    let desc = format!(
        "Your bet: {} <:coin:{COIN}>
        
        **You bet on:** {} ({prediction})
        **{outcome_text}:** {} ({outcome})
        
        {result}
        Your coins: {}",
        bet.format(),
        prediction.emoji(),
        outcome.emoji(),
        coins.format()
    );

    CreateEmbed::<'a>::new()
        .title(title)
        .description(desc)
        .colour(colour)
}
