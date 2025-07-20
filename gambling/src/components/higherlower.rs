use std::collections::HashSet;

use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    Colour, ComponentInteraction, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, EmojiId, Http, parse_emoji,
};
use sqlx::{Database, Pool};
use zayden_core::FormatNum;

use crate::events::{Dispatch, Event, GameEvent};
use crate::games::higherlower::{CARD_TO_NUM, create_embed};
use crate::{CARD_DECK, Coins, GameManager, Gems, GoalsManager, Result, StatsManager};

pub struct HigherLower {
    seq: Vec<String>,
    deck: Vec<EmojiId>,
    payout: i64,
}

impl HigherLower {
    pub async fn higher<
        Db: Database,
        GameHandler: GameManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        StatsHandler: StatsManager<Db>,
    >(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let mut details = HigherLower::from(interaction);
        let prev = details.prev();
        let next = details.next();

        let winner = next.1 >= prev;

        details.seq.push('☝'.into());

        if winner {
            details.payout += 1000
        } else {
            details.seq.push('❌'.into());
        }

        details.seq.push(format!("<:{}:{}>", next.1, next.0));

        if !winner {
            details
                .game_end::<Db, GameHandler, GoalsHandler, StatsHandler>(http, interaction, pool)
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

    pub async fn lower<
        Db: Database,
        GameHandler: GameManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        StatsHandler: StatsManager<Db>,
    >(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let mut details = HigherLower::from(interaction);
        let prev = details.prev();
        let next = details.next();

        let winner = next.1 <= prev;

        details.seq.push('👇'.into());

        if winner {
            details.payout += 1000
        } else {
            details.seq.push('❌'.into());
        }

        details.seq.push(format!("<:{}:{}>", next.1, next.0));

        if !winner {
            details
                .game_end::<Db, GameHandler, GoalsHandler, StatsHandler>(http, interaction, pool)
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

    fn next(&mut self) -> (EmojiId, u8) {
        let emoji = self.deck.pop().unwrap();
        let num = *CARD_TO_NUM.get(&emoji).unwrap();

        (emoji, num)
    }

    fn prev(&self) -> u8 {
        parse_emoji(self.seq.last().unwrap())
            .unwrap()
            .name
            .parse()
            .unwrap()
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
    ) {
        let mut tx = pool.begin().await.unwrap();

        StatsHandler::higherlower(&mut *tx, interaction.user.id, (self.payout / 1000) as i32)
            .await
            .unwrap();

        tx.commit().await.unwrap();

        let mut row = GameHandler::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap();

        row.add_coins(self.payout);

        if self.payout == 52_000 {
            row.add_gems(1);
        }

        let colour = if self.payout > 0 {
            Colour::DARK_GREEN
        } else {
            Colour::RED
        };

        let coins = row.coins_str();

        Dispatch::<Db, GoalsHandler>::new(http, pool)
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

        let seq = seq_line
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
            .iter()
            .copied()
            .filter(|id| !used_cards.contains(id))
            .collect::<Vec<_>>();
        deck.shuffle(&mut rng());

        Self { seq, deck, payout }
    }
}
