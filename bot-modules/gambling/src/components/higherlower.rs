use std::collections::HashSet;

use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    Colour,
    ComponentInteraction,
    Context,
    CreateEmbed,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    EmojiId,
    Http,
    parse_emoji,
};
use sqlx::PgPool;
use tokio::sync::RwLock;
use zayden_core::{EmojiCache, EmojiCacheData, FormatNum};

use crate::components::HigherLowerCustomId;
use crate::events::{Dispatch, Event, GameEvent};
use crate::games::higherlower::create_embed;
use crate::{
    CARD_DECK,
    Coins,
    GamblingError,
    GamblingManager,
    GameDelta,
    GameRow,
    Result,
    StatsManager,
    card_deck,
    card_to_num,
};

pub struct HigherLower {
    seq: Vec<String>,
    deck: Vec<EmojiId>,
    payout: i64,
}

impl HigherLower {
    pub async fn run_components<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let guess = interaction.data.custom_id.parse::<HigherLowerCustomId>()?;

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut details = Self::try_from(interaction)?;
        let prev = details.prev()?;
        let next = if let Some(next) = details.next(&emojis)? {
            next
        } else {
            let mut tx = pool.begin().await?;
            GamblingManager::add_gems(&mut tx, interaction.user.id, 1).await?;
            tx.commit().await?;

            details.new_deck(&emojis)?;
            details.next(&emojis)?.ok_or_else(|| {
                GamblingError::Internal("new deck is empty after reset".to_string())
            })?
        };

        match guess {
            HigherLowerCustomId::Higher => {
                Self::higher(
                    &ctx.http,
                    interaction,
                    pool,
                    &emojis,
                    details,
                    prev,
                    next,
                )
                .await?;
            },
            HigherLowerCustomId::Lower => {
                Self::lower(
                    &ctx.http,
                    interaction,
                    pool,
                    &emojis,
                    details,
                    prev,
                    next,
                )
                .await?;
            },
        }

        Ok(())
    }

    async fn higher(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        emojis: &EmojiCache,
        mut details: Self,
        prev: u8,
        next: (EmojiId, u8),
    ) -> Result<()> {
        let winner = next.1 >= prev;

        if winner {
            details.payout += 1000;
        }

        details.seq.push('☝'.into());

        if !winner {
            details.seq.push('❌'.into());
        }

        details.seq.push(format!("<:{}:{}>", next.1, next.0));

        if !winner {
            details.game_end(http, interaction, pool, emojis).await?;

            return Ok(());
        }

        let embed = create_embed(&details.seq.join(" "), details.payout, winner);

        let msg = if winner {
            CreateInteractionResponseMessage::new().embed(embed)
        } else {
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(Vec::new())
        };

        interaction
            .create_response(http, CreateInteractionResponse::UpdateMessage(msg))
            .await?;

        Ok(())
    }

    async fn lower(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        emojis: &EmojiCache,
        mut details: Self,
        prev: u8,
        next: (EmojiId, u8),
    ) -> Result<()> {
        let winner = next.1 <= prev;

        if winner {
            details.payout += 1000;
        }

        details.seq.push('👇'.into());

        if !winner {
            details.seq.push('❌'.into());
        }

        details.seq.push(format!("<:{}:{}>", next.1, next.0));

        if !winner {
            details.game_end(http, interaction, pool, emojis).await?;

            return Ok(());
        }

        let embed = create_embed(&details.seq.join(" "), details.payout, winner);

        let msg = if winner {
            CreateInteractionResponseMessage::new().embed(embed)
        } else {
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(Vec::new())
        };

        interaction
            .create_response(http, CreateInteractionResponse::UpdateMessage(msg))
            .await?;

        Ok(())
    }

    fn next(&mut self, emojis: &EmojiCache) -> Result<Option<(EmojiId, u8)>> {
        let Some(emoji) = self.deck.pop() else {
            return Ok(None);
        };

        let card_map = card_to_num(emojis)?;
        let num = card_map.get(&emoji).copied().ok_or_else(|| {
            GamblingError::Internal("emoji not in card_to_num map".to_string())
        })?;

        Ok(Some((emoji, num)))
    }

    fn prev(&self) -> Result<u8> {
        let last = self.seq.last().ok_or_else(|| {
            GamblingError::Internal("higher-lower seq is empty".to_string())
        })?;

        let emoji = parse_emoji(last).ok_or_else(|| {
            GamblingError::Internal(
                "last seq element is not a valid discord emoji".to_string(),
            )
        })?;

        emoji.name.parse().map_err(|_e| {
            GamblingError::Internal(
                "emoji name is not a valid card value".to_string(),
            )
        })
    }

    fn new_deck(&mut self, emojis: &EmojiCache) -> Result<()> {
        let deck_ref = if let Some(d) = CARD_DECK.get() {
            d
        } else {
            let new_deck = card_deck(emojis)?;
            let _ = CARD_DECK.set(new_deck);
            CARD_DECK.get().ok_or_else(|| {
                GamblingError::Internal("CARD_DECK init failed".to_string())
            })?
        };
        let mut deck = deck_ref.clone();
        deck.shuffle(&mut rng());
        self.deck = deck;
        Ok(())
    }

    async fn game_end(
        self,
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        emojis: &EmojiCache,
    ) -> Result<()> {
        let mut row = GameRow::get(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let before = row.clone();

        row.add_coins(self.payout);

        let colour = if self.payout > 0 { Colour::DARK_GREEN } else { Colour::RED };

        Dispatch::new(http, pool, emojis)
            .fire(
                interaction.channel_id,
                &mut row,
                Event::Game(GameEvent::new(
                    "higherorlower",
                    interaction.user.id,
                    0,
                    self.payout,
                    self.payout != 0,
                )),
            )
            .await?;

        let delta = GameDelta::between(&before, &row);

        let coins = GameRow::commit(pool, interaction.user.id, &delta)
            .await?
            .ok_or(GamblingError::TransactionConflict)?
            .coins
            .format();

        let mut tx = pool.begin().await?;

        StatsManager::higherlower(
            &mut tx,
            interaction.user.id,
            i32::try_from(self.payout / 1000).unwrap_or(i32::MAX),
        )
        .await?;

        tx.commit().await?;

        let result = format!("Payout: {}", self.payout.format());

        let embed = CreateEmbed::new()
            .title("Higher or Lower")
            .description(format!(
                "{}\n\nYou guessed wrong! Final score: `{}`\n\n{result}\nYour coins: {coins}",
                self.seq.join(" "),
                self.payout / 1000
            ))
            .colour(colour);

        interaction
            .create_response(
                http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(Vec::new()),
                ),
            )
            .await?;

        Ok(())
    }
}

impl TryFrom<&ComponentInteraction> for HigherLower {
    type Error = GamblingError;

    fn try_from(value: &ComponentInteraction) -> Result<Self> {
        let desc = value
            .message
            .as_ref()
            .embeds
            .first()
            .and_then(|embed| embed.description.as_deref())
            .ok_or_else(|| {
                GamblingError::Internal(
                    "higher-lower message missing embed description".to_string(),
                )
            })?;

        let mut lines = desc.lines();
        let seq_line = lines.next().ok_or_else(|| {
            GamblingError::Internal("higher-lower message has no lines".to_string())
        })?;
        let payout_line = lines.nth(1).ok_or_else(|| {
            GamblingError::Internal(
                "higher-lower message missing payout line".to_string(),
            )
        })?;

        let seq = seq_line
            .get(2..)
            .unwrap_or("")
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<_>>();

        let payout = payout_line
            .strip_prefix("Current Payout: ")
            .ok_or_else(|| {
                GamblingError::Internal("payout line missing prefix".to_string())
            })?
            .replace(',', "")
            .parse()
            .map_err(|_e| {
                GamblingError::Internal("payout parse failed".to_string())
            })?;

        let used_cards = seq
            .iter()
            .filter_map(|s| parse_emoji(s))
            .map(|emoji| emoji.id)
            .collect::<HashSet<_>>();

        let mut deck = CARD_DECK
            .get()
            .ok_or_else(|| {
                GamblingError::Internal("CARD_DECK not initialized".to_string())
            })?
            .iter()
            .copied()
            .filter(|id| !used_cards.contains(id))
            .collect::<Vec<_>>();
        deck.shuffle(&mut rng());

        Ok(Self { seq, deck, payout })
    }
}
