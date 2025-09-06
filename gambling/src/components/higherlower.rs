use std::collections::HashSet;

use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    Colour, ComponentInteraction, Context, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, EmojiId, Http, parse_emoji,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCache, EmojiCacheData, FormatNum};

use crate::events::{Dispatch, Event, GameEvent};
use crate::games::higherlower::create_embed;
use crate::{
    CARD_DECK, CARD_TO_NUM, Coins, GamblingManager, GameManager, GameRow, GoalsManager, Result,
    StatsManager, card_deck, card_to_num,
};

pub struct HigherLower {
    seq: Vec<String>,
    deck: Vec<EmojiId>,
    payout: i64,
}

impl HigherLower {
    pub async fn run_components<
        Data: EmojiCacheData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        GameHandler: GameManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        StatsHandler: StatsManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut details = HigherLower::from(interaction);
        let prev = details.prev();
        let next = match details.next(&emojis) {
            Some(next) => next,
            None => {
                let mut tx = pool.begin().await.unwrap();
                GamblingHandler::add_gems(&mut *tx, interaction.user.id, 1)
                    .await
                    .unwrap();
                tx.commit().await.unwrap();

                details.new_deck(&emojis);
                details.next(&emojis).unwrap()
            }
        };

        match interaction.data.custom_id.as_str() {
            "hol_higher" => {
                Self::higher::<Db, GameHandler, GoalsHandler, StatsHandler>(
                    &ctx.http,
                    interaction,
                    pool,
                    &emojis,
                    details,
                    prev,
                    next,
                )
                .await?;
            }
            "hol_lower" => {
                Self::lower::<Db, GameHandler, GoalsHandler, StatsHandler>(
                    &ctx.http,
                    interaction,
                    pool,
                    &emojis,
                    details,
                    prev,
                    next,
                )
                .await?;
            }
            id => unreachable!("Invalid custom_id: {id}"),
        };

        Ok(())
    }

    async fn higher<
        Db: Database,
        GameHandler: GameManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        StatsHandler: StatsManager<Db>,
    >(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        emojis: &EmojiCache,
        mut details: Self,
        prev: u8,
        next: (EmojiId, u8),
    ) -> Result<()> {
        let winner = next.1 >= prev;

        if winner {
            details.payout += 1000
        }

        details.seq.push('☝'.into());

        if !winner {
            details.seq.push('❌'.into());
        }

        details.seq.push(format!("<:{}:{}>", next.1, next.0));

        if !winner {
            details
                .game_end::<Db, GameHandler, GoalsHandler, StatsHandler>(
                    http,
                    interaction,
                    pool,
                    emojis,
                )
                .await;

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

    async fn lower<
        Db: Database,
        GameHandler: GameManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        StatsHandler: StatsManager<Db>,
    >(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        emojis: &EmojiCache,
        mut details: Self,
        prev: u8,
        next: (EmojiId, u8),
    ) -> Result<()> {
        let winner = next.1 <= prev;

        if winner {
            details.payout += 1000
        }

        details.seq.push('👇'.into());

        if !winner {
            details.seq.push('❌'.into());
        }

        details.seq.push(format!("<:{}:{}>", next.1, next.0));

        if !winner {
            details
                .game_end::<Db, GameHandler, GoalsHandler, StatsHandler>(
                    http,
                    interaction,
                    pool,
                    emojis,
                )
                .await;
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

    fn next(&mut self, emojis: &EmojiCache) -> Option<(EmojiId, u8)> {
        let emoji = self.deck.pop()?;
        let num = *CARD_TO_NUM
            .get_or_init(|| card_to_num(emojis))
            .get(&emoji)
            .unwrap();

        Some((emoji, num))
    }

    fn prev(&self) -> u8 {
        parse_emoji(self.seq.last().unwrap())
            .unwrap()
            .name
            .parse()
            .unwrap()
    }

    fn new_deck(&mut self, emojis: &EmojiCache) {
        let mut deck = CARD_DECK.get_or_init(|| card_deck(emojis)).to_vec();

        deck.shuffle(&mut rng());

        self.deck = deck;
    }

    async fn game_end<
        Db: Database,
        GameHandler: GameManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        StatsHandler: StatsManager<Db>,
    >(
        self,
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        emojis: &EmojiCache,
    ) {
        let mut tx = pool.begin().await.unwrap();

        StatsHandler::higherlower(&mut *tx, interaction.user.id, (self.payout / 1000) as i32)
            .await
            .unwrap();

        tx.commit().await.unwrap();

        let mut row = GameHandler::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        row.add_coins(self.payout);

        let colour = if self.payout > 0 {
            Colour::DARK_GREEN
        } else {
            Colour::RED
        };

        let coins = row.coins_str();

        Dispatch::<Db, GoalsHandler>::new(http, pool, emojis)
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
            .await
            .unwrap();

        GameHandler::save(pool, row).await.unwrap();

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
            .await
            .unwrap();
    }
}

impl From<&ComponentInteraction> for HigherLower {
    fn from(value: &ComponentInteraction) -> Self {
        let desc = value
            .message
            .as_ref()
            .embeds
            .first()
            .and_then(|embed| embed.description.as_deref())
            .unwrap();

        let mut lines = desc.lines();
        let seq_line = lines.next().unwrap();
        let payout_line = lines.nth(1).unwrap();

        let seq = seq_line[2..]
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<_>>();

        let payout = payout_line
            .strip_prefix("Current Payout: ")
            .unwrap()
            .replace(',', "")
            .parse()
            .unwrap();

        let used_cards = seq
            .iter()
            .filter_map(|s| parse_emoji(s))
            .map(|emoji| emoji.id)
            .collect::<HashSet<_>>();

        let mut deck = CARD_DECK
            .get()
            .expect("Deck should be initalised at this point")
            .iter()
            .copied()
            .filter(|id| !used_cards.contains(id))
            .collect::<Vec<_>>();
        deck.shuffle(&mut rng());

        Self { seq, deck, payout }
    }
}
